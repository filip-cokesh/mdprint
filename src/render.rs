use anyhow::{Context as _, Result};
use comrak::html::{ChildRendering, format_node_default};
use comrak::nodes::{AstNode, NodeValue};
use comrak::{Options, create_formatter};
use std::fmt::Write as _;

use crate::assets;
use crate::cli::Lang;
use crate::config::Config;
use crate::parse::{FrontMatter, collect_text};

/// Stav vlastního formatteru: číslování obrázků.
#[derive(Default)]
pub struct RenderState {
    figure_prefix: String,
    figure_numbering: bool,
    figures: usize,
}

// Vlastní formatter: `![Popisek](…)` samostatně v odstavci → <figure> + <figcaption>
// s číslováním „Obr. N“. Vše ostatní deleguje na výchozí comrak rendering.
create_formatter!(DocFormatter<RenderState>, {
    NodeValue::Paragraph => |context, node, entering| {
        if is_figure_paragraph(node) {
            if entering {
                context.cr()?;
                context.write_str("<figure>")?;
            } else {
                context.write_str("</figure>")?;
                context.lf()?;
            }
        } else {
            return format_node_default(context, node, entering);
        }
    },
    NodeValue::Image(ref nl) => |context, node, entering| {
        if is_figure_image(node) {
            if entering {
                context.user.figures += 1;
                context.write_str("<img src=\"")?;
                context.escape_href(&nl.url)?;
                context.write_str("\" alt=\"")?;
                context.escape(&collect_text(node))?;
                context.write_str("\" /><figcaption>")?;
                if context.user.figure_numbering {
                    let label = format!("{} {}:", context.user.figure_prefix, context.user.figures);
                    context.write_str("<span class=\"fignum\">")?;
                    context.escape(&label)?;
                    context.write_str("</span> ")?;
                }
                // děti obrázku se vyrenderují jako obsah popisku (včetně kurzívy apod.)
                return Ok(ChildRendering::HTML);
            } else {
                context.write_str("</figcaption>")?;
            }
        } else {
            return format_node_default(context, node, entering);
        }
    },
});

/// Odstavec, jehož jediným dítětem je obrázek s neprázdným popiskem.
fn is_figure_paragraph<'a>(node: &'a AstNode<'a>) -> bool {
    let mut children = node.children();
    match (children.next(), children.next()) {
        (Some(only), None) => is_figure_image(only),
        _ => false,
    }
}

fn is_figure_image<'a>(node: &'a AstNode<'a>) -> bool {
    if !matches!(node.data.borrow().value, NodeValue::Image(_)) {
        return false;
    }
    // popisek = děti uzlu obrázku (alt text)
    let has_caption = node.first_child().is_some();
    let standalone_in_paragraph = node
        .parent()
        .is_some_and(|p| matches!(p.data.borrow().value, NodeValue::Paragraph))
        && node.previous_sibling().is_none()
        && node.next_sibling().is_none();
    has_caption && standalone_in_paragraph
}

/// AST → HTML fragment těla dokumentu (s figure/figcaption).
pub fn body_html<'a>(root: &'a AstNode<'a>, opts: &Options, cfg: &Config) -> Result<String> {
    let mut out = String::new();
    let state = RenderState {
        figure_prefix: cfg.figure_prefix.clone(),
        figure_numbering: cfg.figure_numbering,
        figures: 0,
    };
    DocFormatter::format_document(root, opts, &mut out, state)?;
    Ok(out)
}

/// Kompletní HTML stránka přes minijinja šablonu; CSS, fonty i tělo inline.
pub fn page_html(
    title: &str,
    front_matter: Option<&FrontMatter>,
    cfg: &Config,
    pack: Option<&crate::pack::TemplatePack>,
    toc: Option<&str>,
    body: &str,
) -> Result<String> {
    let mut env = minijinja::Environment::new();
    env.add_template("page.html", assets::TEMPLATE_HTML)
        .context("vadná šablona stránky")?;
    let print_css = env
        .render_str(
            assets::PRINT_CSS,
            minijinja::context! {
                page_size => cfg.paper_size,
                page_margin => cfg.paper_margin,
            },
        )
        .context("vadné print.css")?;

    let lang = match cfg.lang {
        Lang::Cs => "cs",
        Lang::En => "en",
        Lang::De => "de",
    };
    let author = front_matter.and_then(|fm| fm.author.clone());
    let date = front_matter
        .and_then(|fm| fm.date.as_deref())
        .map(|d| format_date(d, cfg.lang));
    let version = front_matter.and_then(|fm| fm.version.as_deref()).map(|v| {
        let label = match cfg.lang {
            Lang::Cs => "verze",
            Lang::En => "version",
            Lang::De => "Version",
        };
        format!("{label} {v}")
    });
    let parts: Vec<String> = [author.clone(), date, version]
        .into_iter()
        .flatten()
        .collect();
    let byline = if parts.is_empty() {
        None
    } else {
        Some(parts.join(" · "))
    };
    // hlavička jen když front matter nese title
    let show_header = front_matter.is_some_and(|fm| fm.title.is_some());

    // branding jen s aktivním packem; údaje firmy: pack defaulty ← toml override
    let company = pack.map(|p| crate::pack::merge_company(&p.company, &cfg.company));
    let company_line = company.as_ref().map(|c| {
        let mut parts: Vec<String> = Vec::new();
        if let Some(a) = &c.address {
            parts.push(a.clone());
        }
        if let Some(i) = &c.ico {
            parts.push(format!("IČ {i}"));
        }
        if let Some(d) = &c.dic {
            parts.push(format!("DIČ {d}"));
        }
        if let Some(w) = &c.web {
            parts.push(w.clone());
        }
        if let Some(e) = &c.email {
            parts.push(e.clone());
        }
        parts.join(" · ")
    });

    let safe = minijinja::value::Value::from_safe_string;
    let page = env
        .get_template("page.html")
        .expect("šablona registrována výše")
        .render(minijinja::context! {
            lang,
            title,
            author,
            byline,
            show_header,
            brand => pack.is_some(),
            company_name => company.as_ref().and_then(|c| c.name.clone()),
            company_line,
            // data URI je bezpečné (base64 abeceda) — bez autoescape entit
            logo_light => pack
                .and_then(|p| p.logo_light.as_deref())
                .map(|b| safe(assets::png_data_uri(b))),
            logo_dark => pack
                .and_then(|p| p.logo_dark.as_deref())
                .map(|b| safe(assets::png_data_uri(b))),
            pack_fonts_css => safe(pack.map(|p| p.fonts_css()).unwrap_or_default()),
            pack_css => safe(pack.map(|p| p.css.clone()).unwrap_or_default()),
            toc => toc.map(|t| safe(t.to_string())),
            fonts_css => safe(assets::fonts_css()),
            katex_fonts_css => safe(assets::katex_fonts_css()),
            vars_css => safe(vars_css(cfg)),
            katex_css => safe(assets::KATEX_CSS.to_string()),
            syntax_css => safe(crate::highlight::Highlighter::css()?),
            screen_css => safe(assets::SCREEN_CSS.to_string()),
            print_css => safe(print_css),
            body => safe(body.to_string()),
        })
        .context("render šablony selhal")?;
    Ok(page)
}

/// CSS proměnné s fontovými stacky; primární rodiny lze přepsat v mdprint.toml
/// sekcí `[fonts]` (embedované fonty zůstávají jako fallback).
fn vars_css(cfg: &Config) -> String {
    format!(
        ":root{{\
         --font-serif:'{serif}','Libertinus Serif',Georgia,'Times New Roman',serif;\
         --font-sans:'{sans}','Libertinus Sans','Segoe UI',Arial,sans-serif;\
         --font-mono:'{mono}','JetBrains Mono',Consolas,monospace;\
         }}\n",
        serif = cfg.font_serif,
        sans = cfg.font_sans,
        mono = cfg.font_mono,
    )
}

/// Obsah (`--toc`): vnořené `<ol>` z nadpisů úrovní 1–3. Kotvy počítá stejný
/// `Anchorizer` jako comrak při renderu — id proto sedí včetně deduplikace,
/// pokud se anchorizují všechny nadpisy ve stejném pořadí.
pub fn build_toc<'a>(root: &'a AstNode<'a>, lang: Lang) -> Option<String> {
    const MAX_LEVEL: u8 = 3;
    let mut anchorizer = comrak::Anchorizer::new();
    let mut entries: Vec<(u8, String, String)> = Vec::new();
    for node in root.descendants() {
        let level = match node.data.borrow().value {
            NodeValue::Heading(h) => h.level,
            _ => continue,
        };
        let text = collect_text(node);
        let id = anchorizer.anchorize(&text);
        if level <= MAX_LEVEL {
            entries.push((level, id, text));
        }
    }
    if entries.is_empty() {
        return None;
    }

    let base = entries.iter().map(|(l, ..)| *l).min().expect("neprázdné");
    let title = match lang {
        Lang::Cs => "Obsah",
        Lang::En => "Contents",
        Lang::De => "Inhalt",
    };
    let mut html = format!("<nav class=\"toc\">\n<h2 class=\"toc-title\">{title}</h2>\n");
    let mut depth = 0u8;
    for (level, id, text) in &entries {
        let target = level.saturating_sub(base) + 1;
        while depth < target {
            html.push_str("<ol>\n");
            depth += 1;
        }
        while depth > target {
            html.push_str("</li>\n</ol>\n");
            depth -= 1;
        }
        if depth == target && !html.ends_with("<ol>\n") {
            html.push_str("</li>\n");
        }
        let mut escaped = String::new();
        // escapování textu položky (id je už jen bezpečný slug z anchorizeru)
        for c in text.chars() {
            match c {
                '&' => escaped.push_str("&amp;"),
                '<' => escaped.push_str("&lt;"),
                '>' => escaped.push_str("&gt;"),
                _ => escaped.push(c),
            }
        }
        let _ = write!(html, "<li><a href=\"#{id}\">{escaped}</a>");
        html.push('\n');
    }
    while depth > 0 {
        html.push_str("</li>\n</ol>\n");
        depth -= 1;
    }
    html.push_str("</nav>\n");
    Some(html)
}

/// ISO datum `RRRR-MM-DD` → jazyková sazba: cs `D. M. RRRR` (s nezlomitelnými
/// mezerami dle ČSN 01 6910), de `DD.MM.RRRR` (DIN 5008); cokoli jiného,
/// a celé `en`, projde beze změny.
fn format_date(date: &str, lang: Lang) -> String {
    if lang == Lang::En {
        return date.to_string();
    }
    let parts: Vec<&str> = date.split('-').collect();
    let [y, m, d] = parts.as_slice() else {
        return date.to_string();
    };
    match (y.parse::<u32>(), m.parse::<u32>(), d.parse::<u32>()) {
        (Ok(y), Ok(m), Ok(d)) if (1..=12).contains(&m) && (1..=31).contains(&d) => {
            let mut out = String::new();
            let _ = match lang {
                Lang::Cs => write!(out, "{d}.\u{a0}{m}.\u{a0}{y}"),
                Lang::De => write!(out, "{d:02}.{m:02}.{y}"),
                Lang::En => unreachable!(),
            };
            out
        }
        _ => date.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn czech_date_from_iso() {
        assert_eq!(format_date("2026-09-02", Lang::Cs), "2.\u{a0}9.\u{a0}2026");
        assert_eq!(format_date("léto 2026", Lang::Cs), "léto 2026");
        assert_eq!(format_date("2026-09-02", Lang::En), "2026-09-02");
    }
}

use anyhow::{Context as _, Result};
use comrak::html::{ChildRendering, format_node_default};
use comrak::nodes::{AstNode, NodeValue};
use comrak::{Options, create_formatter};
use std::fmt::Write as _;

use crate::assets;
use crate::cli::Lang;
use crate::config::Config;
use crate::parse::{FrontMatter, collect_text, is_figure_image, is_figure_paragraph};
use crate::xref::{self, is_table_caption, node_id};

pub use crate::xref::ListEntry;

/// Stav vlastního formatteru: číslování obrázků a tabulek.
#[derive(Default)]
pub struct RenderState {
    figure_prefix: String,
    figure_numbering: bool,
    figures: usize,
    table_prefix: String,
    tables: usize,
}

// Vlastní formatter: `![Popisek](…)` samostatně v odstavci → <figure> + <figcaption>
// s číslováním „Obr. N“; odstavec `Tabulka: …` před tabulkou (označený v xref.rs)
// → <figure class="table"> + <figcaption> „Tab. N“. Vše ostatní deleguje na
// výchozí comrak rendering. Id figur/tabulek přiděluje xref.rs (label nebo
// `fig-N`/`tab-N`); čísla se počítají ve stejném pořadí dokumentu.
create_formatter!(DocFormatter<RenderState>, {
    NodeValue::Paragraph => |context, node, entering| {
        if is_figure_paragraph(node) {
            if entering {
                context.cr()?;
                context.write_str("<figure")?;
                write_id(context, node.first_child())?;
                context.write_str(">")?;
            } else {
                context.write_str("</figure>")?;
                context.lf()?;
            }
        } else if is_table_caption(node) {
            if entering {
                context.user.tables += 1;
                context.cr()?;
                context.write_str("<figure class=\"table\"")?;
                write_id(context, node.next_sibling())?;
                context.write_str("><figcaption><span class=\"tabnum\">")?;
                let label = format!("{} {}:", context.user.table_prefix, context.user.tables);
                context.escape(&label)?;
                context.write_str("</span> ")?;
            } else {
                context.write_str("</figcaption>")?;
                context.lf()?;
            }
        } else {
            return format_node_default(context, node, entering);
        }
    },
    NodeValue::Table(..) => |context, node, entering| {
        let captioned = node.previous_sibling().is_some_and(is_table_caption);
        let rendering = format_node_default(context, node, entering)?;
        if captioned && !entering {
            context.write_str("</figure>")?;
            context.lf()?;
        }
        return Ok(rendering);
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

/// ` id="…"` z atributů uzlu (label nebo pořadové id z xref.rs).
fn write_id<'a>(
    context: &mut comrak::html::Context<RenderState>,
    node: Option<&'a AstNode<'a>>,
) -> std::fmt::Result {
    if let Some(id) = node.and_then(node_id) {
        context.write_str(" id=\"")?;
        context.escape(&id)?;
        context.write_str("\"")?;
    }
    Ok(())
}

/// AST → HTML fragment těla dokumentu (s figure/figcaption).
pub fn body_html<'a>(root: &'a AstNode<'a>, opts: &Options, cfg: &Config) -> Result<String> {
    let mut out = String::new();
    let state = RenderState {
        figure_prefix: cfg.figure_prefix.clone(),
        figure_numbering: cfg.figure_numbering,
        figures: 0,
        table_prefix: cfg.table_prefix.clone(),
        tables: 0,
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
    end_lists: Option<&str>,
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
                page_numbers => cfg.page_numbers,
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

    // lokalizované popisky přepínačů (title atributy; do tisku nejdou)
    let (ui_theme, ui_fontsize) = match cfg.lang {
        Lang::Cs => ("Světlý / tmavý režim", "Velikost písma"),
        Lang::En => ("Light / dark mode", "Font size"),
        Lang::De => ("Heller / dunkler Modus", "Schriftgröße"),
    };

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
    // odkazy patičky z packu; label a url projdou autoescapem šablony
    let links: Vec<minijinja::value::Value> = pack
        .map(|p| p.links.as_slice())
        .unwrap_or_default()
        .iter()
        .map(|l| {
            minijinja::context! {
                label => l.label,
                url => l.url,
                icon => l.icon_data_uri.clone().map(safe),
            }
        })
        .collect();
    let page = env
        .get_template("page.html")
        .expect("šablona registrována výše")
        .render(minijinja::context! {
            lang,
            title,
            author,
            byline,
            show_header,
            ui_theme,
            ui_fontsize,
            font_size => cfg.font_size.name(),
            brand => pack.is_some(),
            company_name => company.as_ref().and_then(|c| c.name.clone()),
            company_line,
            links,
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
            end_lists => end_lists.map(|t| safe(t.to_string())),
            fonts_css => safe(assets::fonts_css()),
            katex_fonts_css => safe(assets::katex_fonts_css()),
            vars_css => safe(vars_css(cfg)),
            katex_css => safe(assets::KATEX_CSS.to_string()),
            syntax_css => safe(crate::highlight::Highlighter::css()?),
            syntax_dark_css => safe(scoped_dark_syntax_css()?),
            screen_css => safe(assets::SCREEN_CSS.to_string()),
            print_css => safe(print_css),
            body => safe(body.to_string()),
        })
        .context("render šablony selhal")?;
    Ok(page)
}

/// Tmavé syntect téma scopované na tmavý režim obrazovky přes CSS nesting
/// (vnořené třídní selektory = potomkové; syntect generuje jen třídy).
/// Tisk zůstává u světlého tématu — oba bloky jsou `@media screen`.
fn scoped_dark_syntax_css() -> Result<String> {
    let dark = crate::highlight::Highlighter::css_dark()?;
    Ok(format!(
        "@media screen and (prefers-color-scheme: dark) {{\n\
         html:not(:has(#theme-invert:checked)) {{\n{dark}}}\n}}\n\
         @media screen and (prefers-color-scheme: light) {{\n\
         html:has(#theme-invert:checked) {{\n{dark}}}\n}}\n"
    ))
}

/// CSS proměnné s fontovými stacky; primární rodiny lze přepsat v mdprint.toml
/// sekcí `[fonts]` (embedované fonty zůstávají jako fallback).
fn vars_css(cfg: &Config) -> String {
    format!(
        ":root{{\
         --font-serif:'{serif}','Libertinus Serif',Georgia,'Times New Roman',serif;\
         --font-sans:'{sans}','Libertinus Sans','Segoe UI',Arial,sans-serif;\
         --font-mono:'{mono}','JetBrains Mono',Consolas,monospace;\
         --math-inline:{inline}em;\
         --math-display:{display}em;\
         }}\n",
        serif = cfg.font_serif,
        sans = cfg.font_sans,
        mono = cfg.font_mono,
        inline = cfg.math_inline,
        display = cfg.math_display,
    )
}

/// Položka obsahu: úroveň nadpisu, id kotvy, text.
#[derive(Debug, Clone)]
pub struct TocEntry {
    pub level: u8,
    pub id: String,
    pub text: String,
}

/// Čísla stran jedné položky obsahu pro malé / střední / velké písmo.
pub type PageTriple = [Option<u32>; 3];

/// Jak vykreslit čísla stran v obsahu a seznamech.
#[derive(Clone)]
pub enum TocPages<'a> {
    /// Bez čísel (prostý `--toc`).
    Off,
    /// Zástupné „000“ pevné šířky — sonda stránkování má stejný layout
    /// jako finální dokument.
    Placeholder,
    /// Změřená čísla podle id kotvy.
    Numbers(&'a std::collections::HashMap<String, PageTriple>),
}

/// Nadpisy úrovní 1–3 pro obsah. Kotvy počítá stejný `Anchorizer` jako
/// comrak při renderu — id proto sedí včetně deduplikace, pokud se
/// anchorizují všechny nadpisy ve stejném pořadí.
pub fn toc_entries<'a>(root: &'a AstNode<'a>) -> Vec<TocEntry> {
    const MAX_LEVEL: u8 = 3;
    let mut anchorizer = comrak::Anchorizer::new();
    let mut entries = Vec::new();
    for node in root.descendants() {
        let level = match node.data.borrow().value {
            NodeValue::Heading(h) => h.level,
            _ => continue,
        };
        let text = collect_text(node);
        let id = anchorizer.anchorize(&text);
        if level <= MAX_LEVEL {
            entries.push(TocEntry { level, id, text });
        }
    }
    entries
}

/// Obsah (`--toc`): vnořené `<ol>`; s `pages` ≠ `Off` nese každá položka
/// vodicí tečky a `<span class="toc-page" data-s data-m data-l>` — print.css
/// zobrazí číslo podle zvolené velikosti písma, na obrazovce je skryté.
pub fn render_toc(entries: &[TocEntry], lang: Lang, pages: TocPages) -> Option<String> {
    if entries.is_empty() {
        return None;
    }
    let base = entries.iter().map(|e| e.level).min().expect("neprázdné");
    let title = match lang {
        Lang::Cs => "Obsah",
        Lang::En => "Contents",
        Lang::De => "Inhalt",
    };
    let class = if matches!(pages, TocPages::Off) {
        "toc"
    } else {
        "toc toc-pages"
    };
    let mut html = format!("<nav class=\"{class}\">\n<h2 class=\"toc-title\">{title}</h2>\n");
    let mut depth = 0u8;
    for entry in entries {
        let target = entry.level.saturating_sub(base) + 1;
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
        // escapování textu položky (id je už jen bezpečný slug z anchorizeru)
        let escaped = escape_text(&entry.text);
        let id = &entry.id;
        let suffix = page_suffix(&pages, id);
        if matches!(pages, TocPages::Off) {
            let _ = write!(html, "<li><a href=\"#{id}\">{escaped}</a>");
        } else {
            let _ = write!(
                html,
                "<li><a href=\"#{id}\"><span class=\"toc-text\">{escaped}</span>{suffix}</a>"
            );
        }
        html.push('\n');
    }
    while depth > 0 {
        html.push_str("</li>\n</ol>\n");
        depth -= 1;
    }
    html.push_str("</nav>\n");
    Some(html)
}

fn escape_text(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            _ => escaped.push(c),
        }
    }
    escaped
}

/// Vodicí tečky + `<span class="toc-page">` podle režimu; `Off` → nic.
fn page_suffix(pages: &TocPages, id: &str) -> String {
    let page_span = |[s, m, l]: [String; 3]| {
        format!(
            "<span class=\"toc-leader\"></span>\
             <span class=\"toc-page\" data-s=\"{s}\" data-m=\"{m}\" data-l=\"{l}\"></span>"
        )
    };
    match pages {
        TocPages::Off => String::new(),
        TocPages::Placeholder => page_span(["000".into(), "000".into(), "000".into()]),
        TocPages::Numbers(map) => {
            let triple = map.get(id).copied().unwrap_or_default();
            page_span(triple.map(|p| p.map(|n| n.to_string()).unwrap_or_default()))
        }
    }
}

/// Seznam obrázků (`--lof`) nebo tabulek (`--lot`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListKind {
    Figures,
    Tables,
}

impl ListKind {
    fn title(self, lang: Lang) -> &'static str {
        match (self, lang) {
            (ListKind::Figures, Lang::Cs) => "Seznam obrázků",
            (ListKind::Figures, Lang::En) => "List of Figures",
            (ListKind::Figures, Lang::De) => "Abbildungsverzeichnis",
            (ListKind::Tables, Lang::Cs) => "Seznam tabulek",
            (ListKind::Tables, Lang::En) => "List of Tables",
            (ListKind::Tables, Lang::De) => "Tabellenverzeichnis",
        }
    }

    fn class(self) -> &'static str {
        match self {
            ListKind::Figures => "lof",
            ListKind::Tables => "lot",
        }
    }
}

/// Plochý seznam `<nav class="toc lof|lot">` se stejným markupem položek jako
/// obsah (`toc-text`, `toc-leader`, `toc-page`), jen místo CSS counteru nese
/// každá položka `<span class="toc-num">Obr. 1</span>`.
pub fn render_list(
    kind: ListKind,
    entries: &[xref::ListEntry],
    lang: Lang,
    pages: TocPages,
) -> Option<String> {
    if entries.is_empty() {
        return None;
    }
    let pages_class = if matches!(pages, TocPages::Off) {
        ""
    } else {
        " toc-pages"
    };
    let mut html = format!(
        "<nav class=\"toc {}{pages_class}\">\n<h2 class=\"toc-title\">{}</h2>\n<ol>\n",
        kind.class(),
        kind.title(lang)
    );
    for entry in entries {
        let _ = writeln!(
            html,
            "<li><a href=\"#{}\"><span class=\"toc-num\">{}</span><span class=\"toc-text\">{}</span>{}</a></li>",
            entry.id,
            escape_text(&entry.label),
            escape_text(&entry.text),
            page_suffix(&pages, &entry.id)
        );
    }
    html.push_str("</ol>\n</nav>\n");
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

    fn entries() -> Vec<TocEntry> {
        vec![
            TocEntry {
                level: 1,
                id: "úvod".into(),
                text: "Úvod & cíle".into(),
            },
            TocEntry {
                level: 2,
                id: "detail".into(),
                text: "Detail".into(),
            },
        ]
    }

    #[test]
    fn toc_without_pages_keeps_plain_links() {
        let html = render_toc(&entries(), Lang::Cs, TocPages::Off).unwrap();
        assert!(html.starts_with("<nav class=\"toc\">"));
        assert!(html.contains("<a href=\"#úvod\">Úvod &amp; cíle</a>"));
        assert!(!html.contains("toc-page"));
    }

    #[test]
    fn toc_placeholder_and_numbers() {
        let html = render_toc(&entries(), Lang::En, TocPages::Placeholder).unwrap();
        assert!(html.starts_with("<nav class=\"toc toc-pages\">"));
        assert_eq!(
            html.matches(r#"data-s="000" data-m="000" data-l="000""#)
                .count(),
            2
        );

        let mut map = std::collections::HashMap::new();
        map.insert("úvod".to_string(), [Some(1), Some(1), Some(2)]);
        let html = render_toc(&entries(), Lang::En, TocPages::Numbers(&map)).unwrap();
        assert!(html.contains(
            r##"<a href="#úvod"><span class="toc-text">Úvod &amp; cíle</span><span class="toc-leader"></span><span class="toc-page" data-s="1" data-m="1" data-l="2"></span></a>"##
        ));
        // chybějící cíl → prázdné číslo, ne pád
        assert!(html.contains(r#"data-s="" data-m="" data-l="""#));
    }

    #[test]
    fn czech_date_from_iso() {
        assert_eq!(format_date("2026-09-02", Lang::Cs), "2.\u{a0}9.\u{a0}2026");
        assert_eq!(format_date("léto 2026", Lang::Cs), "léto 2026");
        assert_eq!(format_date("2026-09-02", Lang::En), "2026-09-02");
    }
}

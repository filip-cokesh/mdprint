//! Statické zvýraznění kódu přes syntect (feature default-fancy, bez C závislostí).
//! Code blocky nahrazuje hotovým HTML (`HtmlBlock`) se `<span class>` tokeny;
//! každý řádek je zabalen do `<span class="line">` kvůli visuté indentaci v tisku.

use anyhow::{Context, Result};
use comrak::nodes::{AstNode, NodeValue};
use syntect::html::{ClassStyle, ClassedHTMLGenerator, css_for_theme_with_class_style};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

const THEME: &str = "InspiredGitHub";
const CLASS_STYLE: ClassStyle = ClassStyle::Spaced;

pub struct Highlighter {
    syntaxes: SyntaxSet,
}

impl Highlighter {
    pub fn new() -> Self {
        Highlighter {
            syntaxes: SyntaxSet::load_defaults_newlines(),
        }
    }

    /// CSS tříd zvýraznění (jedno světlé téma vhodné i pro tisk).
    pub fn css() -> Result<String> {
        let themes = syntect::highlighting::ThemeSet::load_defaults();
        let theme = themes
            .themes
            .get(THEME)
            .with_context(|| format!("téma {THEME} není v syntect defaults"))?;
        css_for_theme_with_class_style(theme, CLASS_STYLE).context("generování CSS tématu selhalo")
    }

    /// Nahradí fenced code blocky zvýrazněným HTML. Neznámý jazyk → prostý
    /// escapovaný výpis (se stejnou strukturou řádků).
    pub fn highlight_all<'a>(&self, root: &'a AstNode<'a>) -> Result<()> {
        for node in root.descendants() {
            let mut data = node.data.borrow_mut();
            let line = data.sourcepos.start.line;
            let NodeValue::CodeBlock(cb) = &data.value else {
                continue;
            };
            let lang = cb.info.split_whitespace().next().unwrap_or("");
            let html = self
                .render_block(lang, &cb.literal)
                .with_context(|| format!("zvýraznění kódu selhalo (řádek {line})"))?;
            data.value = NodeValue::HtmlBlock(comrak::nodes::NodeHtmlBlock {
                literal: html,
                block_type: 0,
            });
        }
        Ok(())
    }

    fn render_block(&self, lang: &str, code: &str) -> Result<String> {
        let class = if lang.is_empty() {
            String::new()
        } else {
            format!(" class=\"language-{lang}\"")
        };
        let body = match self.find_syntax(lang) {
            Some(syntax) => {
                let mut generator =
                    ClassedHTMLGenerator::new_with_class_style(syntax, &self.syntaxes, CLASS_STYLE);
                for src_line in LinesWithEndings::from(code) {
                    generator.parse_html_for_line_which_includes_newline(src_line)?;
                }
                wrap_lines(&generator.finalize())
            }
            None => wrap_lines(&escape_html(code)),
        };
        Ok(format!("<pre><code{class}>{body}</code></pre>\n"))
    }

    fn find_syntax(&self, lang: &str) -> Option<&syntect::parsing::SyntaxReference> {
        if lang.is_empty() {
            return None;
        }
        self.syntaxes
            .find_syntax_by_token(lang)
            .or_else(|| self.syntaxes.find_syntax_by_extension(lang))
    }
}

impl Default for Highlighter {
    fn default() -> Self {
        Self::new()
    }
}

/// Zabalí každý řádek do `<span class="line">…</span>`. Spany syntectu mohou
/// přesahovat konce řádků (víceřádkové stringy apod.), proto se na hranici
/// řádku otevřené spany uzavřou a na začátku dalšího znovu otevřou. Vstup
/// obsahuje jen `<span class="…">`, `</span>` a escapovaný text, `<` se tedy
/// v textu vyskytuje výhradně jako `&lt;`.
fn wrap_lines(html: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut open: Vec<&str> = Vec::new();
    let mut buf = String::new();
    let mut has_text = false;
    let mut i = 0;

    while i < html.len() {
        match html.as_bytes()[i] {
            b'<' => {
                let end = html[i..].find('>').map(|e| i + e + 1).unwrap_or(html.len());
                let tag = &html[i..end];
                if tag.starts_with("</") {
                    open.pop();
                } else {
                    open.push(tag);
                }
                buf.push_str(tag);
                i = end;
            }
            b'\n' => {
                for _ in &open {
                    buf.push_str("</span>");
                }
                lines.push(buf);
                buf = open.concat();
                has_text = false;
                i += 1;
            }
            _ => {
                let next = html[i..].find(['<', '\n']).map_or(html.len(), |n| i + n);
                buf.push_str(&html[i..next]);
                has_text = true;
                i = next;
            }
        }
    }
    // zbytek za posledním \n (typicky jen zavírací spany z finalize) má smysl
    // jen pokud nese text
    if has_text {
        for _ in &open {
            buf.push_str("</span>");
        }
        lines.push(buf);
    }

    let mut out = String::with_capacity(html.len() + lines.len() * 32);
    for line in &lines {
        out.push_str("<span class=\"line\">");
        out.push_str(line);
        out.push_str("</span>\n");
    }
    out
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;
    use comrak::Arena;

    fn html_blocks(source: &str) -> Vec<String> {
        let arena = Arena::new();
        let root = parse::parse(&arena, source, &parse::options());
        Highlighter::new().highlight_all(root).unwrap();
        let mut out = Vec::new();
        for node in root.descendants() {
            if let NodeValue::HtmlBlock(h) = &node.data.borrow().value {
                out.push(h.literal.clone());
            }
        }
        out
    }

    #[test]
    fn rust_block_gets_classed_spans_and_lines() {
        let blocks = html_blocks("```rust\nfn main() {\n    println!(\"ok\");\n}\n```\n");
        assert_eq!(blocks.len(), 1);
        let html = &blocks[0];
        assert!(html.contains("class=\"language-rust\""));
        assert!(html.contains("<span class=\"line\">"));
        assert!(html.contains("source rust"));
        assert_eq!(html.matches("<span class=\"line\">").count(), 3);
    }

    #[test]
    fn unknown_language_is_escaped_plaintext() {
        let blocks = html_blocks("```neznamyjazyk\na < b & c\n```\n");
        assert_eq!(blocks.len(), 1);
        let html = &blocks[0];
        assert!(html.contains("class=\"language-neznamyjazyk\""));
        assert!(html.contains("a &lt; b &amp; c"));
    }

    #[test]
    fn theme_css_generates() {
        let css = Highlighter::css().unwrap();
        assert!(css.contains(".source"));
    }

    #[test]
    fn multiline_string_keeps_spans_balanced_per_line() {
        // víceřádkový Python string — scope přesahuje hranici řádků
        let blocks = html_blocks("```python\ns = \"\"\"první\ndruhý\"\"\"\nx = 1\n```\n");
        let html = &blocks[0];
        assert_eq!(html.matches("<span class=\"line\">").count(), 3);
        assert_eq!(
            html.matches("<span").count(),
            html.matches("</span>").count()
        );
        // prázdné řádky uprostřed bloku se zachovávají
        let blocks = html_blocks("```rust\nlet a = 1;\n\nlet b = 2;\n```\n");
        assert_eq!(blocks[0].matches("<span class=\"line\">").count(), 3);
    }
}

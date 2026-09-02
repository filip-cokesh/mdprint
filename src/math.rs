//! Render matematiky při buildu přes katex-rs (nativní Rust port KaTeXu).
//! Uzly `NodeValue::Math` nahrazuje hotovým HTML (`HtmlInline`), které pak
//! projde renderem beze změny.

use anyhow::{Result, anyhow};
use comrak::nodes::{AstNode, NodeValue};
use katex::KatexContext;
use katex::types::{OutputFormat, Settings};

pub struct MathRenderer {
    ctx: KatexContext,
    inline: Settings,
    display: Settings,
}

impl MathRenderer {
    pub fn new() -> Self {
        let settings = |display_mode: bool| {
            Settings::builder()
                .display_mode(display_mode)
                .output(OutputFormat::HtmlAndMathml)
                .build()
        };
        MathRenderer {
            ctx: KatexContext::default(),
            inline: settings(false),
            display: settings(true),
        }
    }

    /// Nahradí všechny math uzly vyrenderovaným HTML. Chybný TeX = chyba buildu
    /// s číslem řádku; žádné tiché degradace.
    pub fn render_all<'a>(&self, root: &'a AstNode<'a>) -> Result<()> {
        for node in root.descendants() {
            let mut data = node.data.borrow_mut();
            let line = data.sourcepos.start.line;
            let NodeValue::Math(m) = &data.value else {
                continue;
            };
            let settings = if m.display_math {
                &self.display
            } else {
                &self.inline
            };
            let html = katex::render_to_string(&self.ctx, &m.literal, settings)
                .map_err(|e| anyhow!("chyba KaTeX (řádek {line}): {e}"))?;
            data.value = NodeValue::HtmlInline(html);
        }
        Ok(())
    }
}

impl Default for MathRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;
    use comrak::Arena;

    #[test]
    fn renders_inline_and_display_math() {
        let arena = Arena::new();
        let root = parse::parse(
            &arena,
            "Vzorec $E = mc^2$ a blok:\n\n$$\\frac{a}{b}$$\n",
            &parse::options(),
        );
        MathRenderer::new().render_all(root).unwrap();

        let mut htmls = Vec::new();
        for node in root.descendants() {
            if let NodeValue::HtmlInline(h) = &node.data.borrow().value {
                htmls.push(h.clone());
            }
        }
        assert_eq!(htmls.len(), 2);
        assert!(htmls[0].contains("class=\"katex\""));
        assert!(htmls[1].contains("katex-display"));
    }

    #[test]
    fn invalid_tex_fails_with_line() {
        let arena = Arena::new();
        let root = parse::parse(
            &arena,
            "text\n\nchybný $\\frac{a$ vzorec\n",
            &parse::options(),
        );
        let err = MathRenderer::new().render_all(root).unwrap_err();
        assert!(err.to_string().contains("řádek 3"), "{err}");
    }
}

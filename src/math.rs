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
    /// s číslem řádku; žádné tiché degradace. Vzorce širší než tisková stránka
    /// dostanou třídu `math-wide-N`, kterou print.css zmenší (KaTeX display
    /// math neumí automaticky lámat).
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
            let mut html = katex::render_to_string(&self.ctx, &m.literal, settings)
                .map_err(|e| anyhow!("chyba KaTeX (řádek {line}): {e}"))?;
            html = tag_wide_math(&html, &m.literal, m.display_math);
            data.value = NodeValue::HtmlInline(html);
        }
        Ok(())
    }
}

/// Šířkový rozpočet tisku: A4 − okraje = 17 cm; display katex ≈ 11,3 pt/em
/// (10,5 pt × 1,08) → ~42 em na řádek.
const DISPLAY_BUDGET_EM: f32 = 42.0;
/// Inline vzorec delší než ~36 em (většina řádku) se v tisku mírně zmenší.
const INLINE_BUDGET_EM: f32 = 36.0;

/// Přidá třídu `math-wide-N` podle odhadované šířky vzorce.
fn tag_wide_math(html: &str, tex: &str, display: bool) -> String {
    let width = estimate_width_em(tex);
    if display {
        let tier = match width {
            w if w > 65.0 => 3,
            w if w > 52.0 => 2,
            w if w > DISPLAY_BUDGET_EM => 1,
            _ => return html.to_string(),
        };
        html.replacen(
            "class=\"katex-display\"",
            &format!("class=\"katex-display math-wide-{tier}\""),
            1,
        )
    } else if width > INLINE_BUDGET_EM {
        html.replacen("class=\"katex\"", "class=\"katex math-wide-1\"", 1)
    } else {
        html.to_string()
    }
}

/// Hrubý odhad sazební šířky TeX výrazu v em. Nemusí být přesný — slouží jen
/// k roztřídění do tierů zmenšení pro tisk; kalibrováno na reálných vzorcích
/// (viz testy).
pub(crate) fn estimate_width_em(tex: &str) -> f32 {
    let chars: Vec<char> = tex.chars().collect();
    let (w, _) = walk(&chars, 0, None);
    w
}

/// Projde `chars` od `i` po zarážku (None = konec); vrací (šířka, další index).
fn walk(chars: &[char], mut i: usize, stop: Option<char>) -> (f32, usize) {
    let mut w = 0.0f32;
    while i < chars.len() {
        let c = chars[i];
        if Some(c) == stop {
            return (w, i + 1);
        }
        match c {
            '{' => {
                let (gw, ni) = walk(chars, i + 1, Some('}'));
                w += gw;
                i = ni;
            }
            '\\' => {
                let (cw, ni) = command(chars, i + 1);
                w += cw;
                i = ni;
            }
            '^' | '_' => {
                let (sw, ni) = atom(chars, i + 1);
                w += 0.6 * sw;
                i = ni;
            }
            '+' | '-' | '=' | '<' | '>' => {
                w += 1.1;
                i += 1;
            }
            '(' | ')' | '[' | ']' | '|' => {
                w += 0.5;
                i += 1;
            }
            ',' | '.' | ';' | ':' | '/' | '*' | '!' | '\'' => {
                w += 0.35;
                i += 1;
            }
            ' ' | '\n' | '\t' | '\r' | '&' | '}' => {
                i += 1;
            }
            _ => {
                w += 0.55;
                i += 1;
            }
        }
    }
    (w, i)
}

/// Jeden atom za `^`/`_` nebo za `\left`: skupina, příkaz, nebo znak.
fn atom(chars: &[char], i: usize) -> (f32, usize) {
    match chars.get(i) {
        Some('{') => walk(chars, i + 1, Some('}')),
        Some('\\') => command(chars, i + 1),
        Some(_) => (0.55, i + 1),
        None => (0.0, i),
    }
}

/// Zpracuje `\command` (index ukazuje ZA zpětné lomítko).
fn command(chars: &[char], i: usize) -> (f32, usize) {
    // jednoznakové příkazy (\, \; \! \\ …)
    let Some(&first) = chars.get(i) else {
        return (0.0, i);
    };
    if !first.is_ascii_alphabetic() {
        let w = match first {
            ',' | '!' => 0.15,
            ';' | ':' => 0.25,
            _ => 0.0,
        };
        return (w, i + 1);
    }
    let mut j = i;
    while j < chars.len() && chars[j].is_ascii_alphabetic() {
        j += 1;
    }
    let name: String = chars[i..j].iter().collect();
    match name.as_str() {
        "frac" | "dfrac" | "tfrac" | "binom" => {
            let (a, ni) = atom(chars, j);
            let (b, ni2) = atom(chars, ni);
            (a.max(b) + 0.3, ni2)
        }
        "sqrt" => {
            let (a, ni) = atom(chars, j);
            (a + 1.0, ni)
        }
        "left" | "right" => {
            // následuje oddělovač (znak nebo \příkaz)
            let (_, ni) = atom(chars, j);
            (0.5, ni)
        }
        "sum" | "prod" | "int" | "oint" | "iint" => (1.8, j),
        "quad" => (2.0, j),
        "qquad" => (4.0, j),
        "cdot" => (0.6, j),
        "times" | "div" | "pm" | "mp" => (1.0, j),
        "approx" | "le" | "ge" | "ne" | "leq" | "geq" | "sim" | "equiv" | "to" | "rightarrow"
        | "Rightarrow" | "propto" => (1.6, j),
        // stylové příkazy šířku nemění — skupina se změří normálně
        "text" | "mathrm" | "mathbf" | "mathit" | "mathsf" | "mathbb" | "mathcal" | "mathfrak"
        | "operatorname" | "textbf" | "textit" => (0.0, j),
        "max" | "min" | "sin" | "cos" | "tan" | "log" | "ln" | "exp" | "lim" => {
            (0.55 * name.chars().count() as f32, j)
        }
        // řecká písmena a neznámé příkazy ≈ jeden glyf
        _ => (0.8, j),
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
    fn width_estimate_tiers() {
        // dlouhý vzorec z přednášky (řetěz zlomků v \left(\right)) → tier
        let long = r"\Delta l = \frac{1}{210 \cdot 10^9} \left( \frac{-300 \cdot 10^3 \cdot 3{,}4}{7{,}38 \cdot 10^{-3}} + \frac{-700 \cdot 10^3 \cdot 3{,}6}{7{,}38 \cdot 10^{-3}} + \frac{-1100 \cdot 10^3 \cdot 3{,}6}{1{,}016 \cdot 10^{-2}} + \frac{-1500 \cdot 10^3 \cdot 3{,}6}{1{,}016 \cdot 10^{-2}} + \frac{-1900 \cdot 10^3 \cdot 3{,}6}{1{,}414 \cdot 10^{-2}} + \frac{-2400 \cdot 10^3 \cdot 3{,}8}{1{,}414 \cdot 10^{-2}} \right)";
        let w = estimate_width_em(long);
        assert!(w > 42.0, "dlouhý vzorec musí spadnout do tieru: {w}");

        // krátké vzorce zůstávají bez zásahu
        let short = r"w_{\max} = \frac{5\,q\,L^4}{384\,E\,I}";
        let w = estimate_width_em(short);
        assert!(w < 42.0, "krátký vzorec nesmí do tieru: {w}");

        let inline = r"\sigma_6 = N_6/A_6 = -2400 \cdot 10^3/(1{,}414 \cdot 10^{-2}) \approx -170 \text{ MPa}";
        let w = estimate_width_em(inline);
        assert!(w < 36.0, "běžný inline vzorec bez zmenšení: {w}");
    }

    #[test]
    fn wide_class_injected_only_for_wide_display() {
        let arena = Arena::new();
        let root = parse::parse(
            &arena,
            "$$\\frac{a}{b}$$\n\n$$a_1 + a_2 + a_3 + a_4 + a_5 + a_6 + a_7 + a_8 + a_9 + b_1 + b_2 + b_3 + b_4 + b_5 + b_6 + b_7 + b_8 + b_9 + c_1 + c_2 + c_3 + c_4 + c_5 + c_6 + c_7 + c_8 + c_9 + d_1$$\n",
            &parse::options(),
        );
        MathRenderer::new().render_all(root).unwrap();
        let mut htmls = Vec::new();
        for node in root.descendants() {
            if let NodeValue::HtmlInline(h) = &node.data.borrow().value {
                htmls.push(h.clone());
            }
        }
        assert!(!htmls[0].contains("math-wide"), "krátký display bez třídy");
        assert!(
            htmls[1].contains("class=\"katex-display math-wide-"),
            "široký display musí dostat třídu"
        );
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

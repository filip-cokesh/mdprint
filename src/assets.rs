//! Všechny assety embedované do binárky (`include_str!`/`include_bytes!`);
//! za běhu se z disku nečte nic.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;

pub const TEMPLATE_HTML: &str = include_str!("../assets/template.html");
pub const SCREEN_CSS: &str = include_str!("../assets/screen.css");
pub const PRINT_CSS: &str = include_str!("../assets/print.css");

/// KaTeX 0.18.5 CSS s odstraněnými `@font-face` bloky (nahrazují je inlinované
/// WOFF2 z `katex_fonts_css`).
pub const KATEX_CSS: &str = include_str!("../assets/katex.css");

/// PNG jako data URI pro `<img src>`.
pub fn png_data_uri(bytes: &[u8]) -> String {
    format!("data:image/png;base64,{}", B64.encode(bytes))
}

pub struct FontFace {
    pub family: &'static str,
    pub weight: u16,
    pub style: &'static str,
    pub bytes: &'static [u8],
}

macro_rules! face {
    ($family:literal, $weight:literal, $style:literal, $file:literal) => {
        FontFace {
            family: $family,
            weight: $weight,
            style: $style,
            bytes: include_bytes!(concat!("../assets/fonts/", $file)),
        }
    };
}

/// Libertinus 7.051 a JetBrains Mono 2.304, WOFF2, licence OFL.
pub const FONT_FACES: &[FontFace] = &[
    face!(
        "Libertinus Serif",
        400,
        "normal",
        "LibertinusSerif-Regular.woff2"
    ),
    face!(
        "Libertinus Serif",
        400,
        "italic",
        "LibertinusSerif-Italic.woff2"
    ),
    face!(
        "Libertinus Serif",
        700,
        "normal",
        "LibertinusSerif-Bold.woff2"
    ),
    face!(
        "Libertinus Serif",
        700,
        "italic",
        "LibertinusSerif-BoldItalic.woff2"
    ),
    face!(
        "Libertinus Sans",
        400,
        "normal",
        "LibertinusSans-Regular.woff2"
    ),
    face!(
        "Libertinus Sans",
        400,
        "italic",
        "LibertinusSans-Italic.woff2"
    ),
    face!(
        "Libertinus Sans",
        700,
        "normal",
        "LibertinusSans-Bold.woff2"
    ),
    face!(
        "JetBrains Mono",
        400,
        "normal",
        "JetBrainsMono-Regular.woff2"
    ),
    face!(
        "JetBrains Mono",
        400,
        "italic",
        "JetBrainsMono-Italic.woff2"
    ),
    face!("JetBrains Mono", 700, "normal", "JetBrainsMono-Bold.woff2"),
];

/// KaTeX 0.18.5 WOFF2 fonty; family/řez se odvozují ze jména souboru.
pub const KATEX_FONT_FILES: &[(&str, &[u8])] = &[
    (
        "KaTeX_AMS-Regular",
        include_bytes!("../assets/katex-fonts/KaTeX_AMS-Regular.woff2"),
    ),
    (
        "KaTeX_Caligraphic-Bold",
        include_bytes!("../assets/katex-fonts/KaTeX_Caligraphic-Bold.woff2"),
    ),
    (
        "KaTeX_Caligraphic-Regular",
        include_bytes!("../assets/katex-fonts/KaTeX_Caligraphic-Regular.woff2"),
    ),
    (
        "KaTeX_Fraktur-Bold",
        include_bytes!("../assets/katex-fonts/KaTeX_Fraktur-Bold.woff2"),
    ),
    (
        "KaTeX_Fraktur-Regular",
        include_bytes!("../assets/katex-fonts/KaTeX_Fraktur-Regular.woff2"),
    ),
    (
        "KaTeX_Main-Bold",
        include_bytes!("../assets/katex-fonts/KaTeX_Main-Bold.woff2"),
    ),
    (
        "KaTeX_Main-BoldItalic",
        include_bytes!("../assets/katex-fonts/KaTeX_Main-BoldItalic.woff2"),
    ),
    (
        "KaTeX_Main-Italic",
        include_bytes!("../assets/katex-fonts/KaTeX_Main-Italic.woff2"),
    ),
    (
        "KaTeX_Main-Regular",
        include_bytes!("../assets/katex-fonts/KaTeX_Main-Regular.woff2"),
    ),
    (
        "KaTeX_Math-BoldItalic",
        include_bytes!("../assets/katex-fonts/KaTeX_Math-BoldItalic.woff2"),
    ),
    (
        "KaTeX_Math-Italic",
        include_bytes!("../assets/katex-fonts/KaTeX_Math-Italic.woff2"),
    ),
    (
        "KaTeX_SansSerif-Bold",
        include_bytes!("../assets/katex-fonts/KaTeX_SansSerif-Bold.woff2"),
    ),
    (
        "KaTeX_SansSerif-Italic",
        include_bytes!("../assets/katex-fonts/KaTeX_SansSerif-Italic.woff2"),
    ),
    (
        "KaTeX_SansSerif-Regular",
        include_bytes!("../assets/katex-fonts/KaTeX_SansSerif-Regular.woff2"),
    ),
    (
        "KaTeX_Script-Regular",
        include_bytes!("../assets/katex-fonts/KaTeX_Script-Regular.woff2"),
    ),
    (
        "KaTeX_Size1-Regular",
        include_bytes!("../assets/katex-fonts/KaTeX_Size1-Regular.woff2"),
    ),
    (
        "KaTeX_Size2-Regular",
        include_bytes!("../assets/katex-fonts/KaTeX_Size2-Regular.woff2"),
    ),
    (
        "KaTeX_Size3-Regular",
        include_bytes!("../assets/katex-fonts/KaTeX_Size3-Regular.woff2"),
    ),
    (
        "KaTeX_Size4-Regular",
        include_bytes!("../assets/katex-fonts/KaTeX_Size4-Regular.woff2"),
    ),
    (
        "KaTeX_Typewriter-Regular",
        include_bytes!("../assets/katex-fonts/KaTeX_Typewriter-Regular.woff2"),
    ),
];

/// `@font-face` blok s WOFF2 jako base64 data URI (používá i modul `pack`).
pub(crate) fn font_face_css(family: &str, weight: u16, style: &str, bytes: &[u8]) -> String {
    format!(
        "@font-face{{font-family:'{family}';font-weight:{weight};font-style:{style};\
         src:url(data:font/woff2;base64,{}) format('woff2');}}\n",
        B64.encode(bytes),
    )
}

/// `@font-face` bloky s WOFF2 inlinovaným jako base64 data URI.
pub fn fonts_css() -> String {
    let mut css = String::new();
    for f in FONT_FACES {
        css.push_str(&font_face_css(f.family, f.weight, f.style, f.bytes));
    }
    css
}

/// `@font-face` bloky pro KaTeX fonty; `KaTeX_Main-BoldItalic` → family
/// `KaTeX_Main`, weight 700, style italic.
pub fn katex_fonts_css() -> String {
    let mut css = String::new();
    for (name, bytes) in KATEX_FONT_FILES {
        let (family, variant) = name.split_once('-').expect("jméno KaTeX fontu bez '-'");
        let weight = if variant.contains("Bold") { 700 } else { 400 };
        let style = if variant.contains("Italic") {
            "italic"
        } else {
            "normal"
        };
        css.push_str(&font_face_css(family, weight, style, bytes));
    }
    css
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fonts_css_has_all_faces() {
        let css = fonts_css();
        assert_eq!(css.matches("@font-face").count(), FONT_FACES.len());
        assert!(css.contains("Libertinus Serif"));
        assert!(css.contains("JetBrains Mono"));
        // WOFF2 magic "wOF2" v base64 začíná "d09G"
        assert_eq!(css.matches("base64,d09G").count(), FONT_FACES.len());
    }
}

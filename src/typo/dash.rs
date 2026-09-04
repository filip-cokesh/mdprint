//! Pomlčky a rozsahy. Spojovník uvnitř slov (`modro-zelený`) zůstává.

use std::sync::LazyLock;

use fancy_regex::Regex;

/// ` - ` → `&nbsp;– ` (en dash). Mezera před pomlčkou je nezlomitelná —
/// pomlčka nesmí začínat řádek (ČSN 01 6910).
pub fn spaced_dash(text: &str) -> String {
    static RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?<=\S)[ \u{a0}][-–][ \u{a0}](?=\S)").expect("vadný regex"));
    RE.replace_all(text, "\u{a0}– ").into_owned()
}

/// EN (Chicago): ` - `, ` -- ` i `--` mezi slovy → em dash `—` přisazený
/// ke slovům (word—word). Spojovník uvnitř slov zůstává.
pub fn em_dash(text: &str) -> String {
    static SPACED: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?<=\S)[ \u{a0}](?:--?|–|—)[ \u{a0}](?=\S)").expect("vadný regex")
    });
    static TIGHT: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?<=\p{L})--(?=\p{L})").expect("vadný regex"));
    let s = SPACED.replace_all(text, "\u{2014}").into_owned();
    TIGHT.replace_all(&s, "\u{2014}").into_owned()
}

/// Číselný rozsah `10-20` → `10–20` (en dash bez mezer).
pub fn number_range(text: &str) -> String {
    static RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?<=\d)-(?=\d)").expect("vadný regex"));
    RE.replace_all(text, "–").into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[track_caller]
    fn table(f: fn(&str) -> String, cases: &[(&str, &str)]) {
        for (input, expected) in cases {
            assert_eq!(&f(input), expected, "vstup: {input:?}");
        }
    }

    #[test]
    fn dashes() {
        table(
            spaced_dash,
            &[
                ("Praha - Brno", "Praha\u{a0}– Brno"),
                ("text - vsuvka - text", "text\u{a0}– vsuvka\u{a0}– text"),
                // již správná pomlčka dostane aspoň nezlomitelnou mezeru
                ("Praha – Brno", "Praha\u{a0}– Brno"),
                // spojovník uvnitř slova se nemění
                ("modro-zelený", "modro-zelený"),
                ("je-li", "je-li"),
            ],
        );
    }

    #[test]
    fn em_dashes() {
        table(
            em_dash,
            &[
                ("word - word", "word\u{2014}word"),
                ("word -- word", "word\u{2014}word"),
                ("word--word", "word\u{2014}word"),
                ("wait – no", "wait\u{2014}no"),
                // spojovník uvnitř slova zůstává
                ("well-known", "well-known"),
                ("state-of-the-art", "state-of-the-art"),
            ],
        );
    }

    #[test]
    fn ranges() {
        table(
            number_range,
            &[
                ("10-20", "10–20"),
                ("strany 5-8 a 9-12", "strany 5–8 a 9–12"),
                ("1918-1938", "1918–1938"),
                ("modro-zelený", "modro-zelený"),
            ],
        );
    }
}

//! Výpustka, znak násobení, tisícové skupiny.

use std::sync::LazyLock;

use fancy_regex::Regex;

/// `...` → `…`.
pub fn ellipsis(text: &str) -> String {
    text.replace("...", "\u{2026}")
}

/// `x` mezi čísly → `×`; mezery kolem se stávají nezlomitelnými.
pub fn multiply_sign(text: &str) -> String {
    static SPACED: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?<=\d)[ \u{a0}]x[ \u{a0}](?=\d)").expect("vadný regex"));
    // `(?<!\b0)` chrání hexadecimální zápisy typu 0x1F
    static TIGHT: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?<=\d)(?<!\b0)x(?=\d)").expect("vadný regex"));
    let s = SPACED.replace_all(text, "\u{a0}×\u{a0}").into_owned();
    TIGHT.replace_all(&s, "×").into_owned()
}

/// Existující mezery v tisícových skupinách (`1 000 000`) → úzká nezlomitelná
/// U+202F. Do holých číslic (`10000`) se nezasahuje — mohlo by jít o letopočet
/// či identifikátor.
pub fn thousands_groups(text: &str) -> String {
    static RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?<=\d) (?=\d{3}(?:\D|$))").expect("vadný regex"));
    // víc průchodů kvůli navazujícím skupinám (1 000 000)
    let mut s = RE.replace_all(text, "\u{202f}").into_owned();
    loop {
        let next = RE.replace_all(&s, "\u{202f}").into_owned();
        if next == s {
            return s;
        }
        s = next;
    }
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
    fn ellipsis_rule() {
        table(
            ellipsis,
            &[
                ("a tak dále...", "a tak dále…"),
                ("a... b", "a… b"),
                ("beze změny.", "beze změny."),
            ],
        );
    }

    #[test]
    fn multiply() {
        table(
            multiply_sign,
            &[
                ("2 x 3", "2\u{a0}×\u{a0}3"),
                ("40x60 cm", "40×60 cm"),
                ("2 x 3 x 4", "2\u{a0}×\u{a0}3\u{a0}×\u{a0}4"),
                // „x“ ve slově zůstává
                ("axb", "axb"),
                ("text x text", "text x text"),
                // hexadecimální zápis se nemění
                ("0x1F", "0x1F"),
            ],
        );
    }

    #[test]
    fn thousands() {
        table(
            thousands_groups,
            &[
                ("1 000", "1\u{202f}000"),
                ("1 000 000 Kč", "1\u{202f}000\u{202f}000 Kč"),
                ("10 500,75", "10\u{202f}500,75"),
                // holé číslice se nemění
                ("10000", "10000"),
                // dvouciferná skupina není tisícová
                ("v roce 20 22", "v roce 20 22"),
            ],
        );
    }
}

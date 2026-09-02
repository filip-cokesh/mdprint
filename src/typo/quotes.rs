//! Uvozovky: české „…“ a vnořené ‚…‘ z rovných `"` a `'`; anglické “…” ‘…’.
//! Párování probíhá uvnitř jednoho textového uzlu — uvozovky přes hranici
//! zvýraznění (`"text **tučně**"`) se nepárují (vědomé omezení v1).

use std::sync::LazyLock;

use fancy_regex::Regex;

static DOUBLE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?<=^|[\s\u{a0}(\[{–—>])"(\S(?:[^"]*\S)?)"(?=$|[\s\u{a0}.,;:!?)\]}<–—])"#)
        .expect("vadný regex")
});

static SINGLE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?<=^|[\s\u{a0}(\[{„“–—>])'(\S(?:[^']*\S)?)'(?=$|[\s\u{a0}.,;:!?)\]}<„“–—])"#)
        .expect("vadný regex")
});

pub fn czech_quotes(text: &str) -> String {
    let s = DOUBLE.replace_all(text, "\u{201e}$1\u{201c}").into_owned();
    SINGLE.replace_all(&s, "\u{201a}$1\u{2018}").into_owned()
}

pub fn english_quotes(text: &str) -> String {
    let s = DOUBLE.replace_all(text, "\u{201c}$1\u{201d}").into_owned();
    SINGLE.replace_all(&s, "\u{2018}$1\u{2019}").into_owned()
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
    fn czech() {
        table(
            czech_quotes,
            &[
                ("řekl \"ahoj\" a šel", "řekl „ahoj“ a šel"),
                ("\"Celá věta.\"", "„Celá věta.“"),
                (
                    "dvě \"slova\" a \"ještě\" jednou",
                    "dvě „slova“ a „ještě“ jednou",
                ),
                ("vnořené 'царь' slovo", "vnořené ‚царь‘ slovo"),
                ("\"vně 'uvnitř' vně\"", "„vně ‚uvnitř‘ vně“"),
                // apostrof uvnitř slova zůstává
                ("d'Artagnan", "d'Artagnan"),
                // palce/metry — nepárová uvozovka zůstává
                ("deska 5\" široká", "deska 5\" široká"),
            ],
        );
    }

    #[test]
    fn english() {
        table(
            english_quotes,
            &[
                ("he said \"hi\" there", "he said \u{201c}hi\u{201d} there"),
                ("nested 'word' here", "nested \u{2018}word\u{2019} here"),
            ],
        );
    }
}

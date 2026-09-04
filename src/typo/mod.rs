//! Česká mikrotypografie nad AST. Transformuje **výhradně** `NodeValue::Text` —
//! kód, matematika (po fázi 3 už `HtmlInline`), URL ani atributy se nedotýká.
//! Pravidla dle Peciny (*Knihy a typografie*) a ČSN 01 6910; každé pravidlo je
//! samostatná funkce s tabulkovými testy ve svém modulu.

pub mod dash;
pub mod misc;
pub mod nbsp;
pub mod quotes;

use comrak::nodes::{AstNode, NodeValue};

use crate::cli::Lang;

/// Pořadí je záměrné: uvozovky dřív než pomlčky (aby `"a" - "b"` nezmátlo
/// párování), apostrof až po spárování jednoduchých uvozovek, tisícové
/// skupiny až po jednotkách (obě pracují s číslicemi).
pub fn transform(text: &str, lang: Lang) -> String {
    match lang {
        Lang::Cs => {
            let s = quotes::double_low_quotes(text);
            let s = misc::apostrophe(&s);
            let s = misc::ellipsis(&s);
            let s = dash::spaced_dash(&s);
            let s = dash::number_range(&s);
            let s = misc::multiply_sign(&s);
            let s = nbsp::single_letter_prepositions(&s);
            let s = nbsp::number_unit(&s);
            let s = nbsp::abbreviations(&s);
            let s = nbsp::initials(&s);
            let s = nbsp::dates(&s);
            let s = nbsp::paragraph_sign(&s);
            misc::thousands_groups(&s)
        }
        Lang::En => {
            let s = quotes::english_quotes(text);
            let s = misc::apostrophe(&s);
            let s = misc::ellipsis(&s);
            let s = dash::em_dash(&s);
            let s = dash::number_range(&s);
            let s = misc::multiply_sign(&s);
            // číslo–jednotka je SI konvence, ne čeština; tisícové skupiny
            // angličtina píše čárkami — U+202F pravidlo se nepoužije
            nbsp::number_unit(&s)
        }
        Lang::De => {
            let s = quotes::double_low_quotes(text);
            let s = misc::apostrophe(&s);
            let s = misc::ellipsis(&s);
            // Gedankenstrich = spaced en dash, jako čeština
            let s = dash::spaced_dash(&s);
            let s = dash::number_range(&s);
            let s = misc::multiply_sign(&s);
            let s = nbsp::number_unit(&s);
            let s = nbsp::german_abbreviations(&s);
            let s = nbsp::dates_de(&s);
            let s = nbsp::paragraph_sign(&s);
            misc::thousands_groups(&s)
        }
    }
}

/// Aplikuje typografii na všechny textové uzly dokumentu.
pub fn apply<'a>(root: &'a AstNode<'a>, lang: Lang) {
    for node in root.descendants() {
        let mut data = node.data.borrow_mut();
        if let NodeValue::Text(t) = &mut data.value {
            let transformed = transform(t, lang);
            if transformed != *t.as_ref() {
                *t = transformed.into();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;
    use comrak::Arena;

    /// Kód a matematika zůstávají netknuté, text se transformuje.
    #[test]
    fn only_text_nodes_are_touched() {
        let arena = Arena::new();
        let root = parse::parse(
            &arena,
            "Jdu k lesu s `k lesu v kódu` a $k + v$ vzorcem.\n",
            &parse::options(),
        );
        apply(root, Lang::Cs);

        let mut texts = String::new();
        let mut code = String::new();
        let mut math = String::new();
        for node in root.descendants() {
            match &node.data.borrow().value {
                NodeValue::Text(t) => texts.push_str(t),
                NodeValue::Code(c) => code.push_str(&c.literal),
                NodeValue::Math(m) => math.push_str(&m.literal),
                _ => {}
            }
        }
        assert!(texts.contains("k\u{a0}lesu"), "{texts:?}");
        assert!(texts.contains("s\u{a0}"), "{texts:?}");
        assert_eq!(code, "k lesu v kódu");
        assert_eq!(math, "k + v");
    }

    #[test]
    fn english_pipeline() {
        assert_eq!(
            transform("He said \"hello\" to k friend...", Lang::En),
            "He said \u{201c}hello\u{201d} to k friend\u{2026}",
            "žádná česká pravidla (k se neváže)"
        );
        assert_eq!(
            transform("don't stop - see pages 10-20, load 10 kN", Lang::En),
            "don\u{2019}t stop\u{2014}see pages 10\u{2013}20, load 10\u{a0}kN"
        );
        assert_eq!(
            transform("1 000 000 cycles", Lang::En),
            "1 000 000 cycles",
            "tisícové U+202F se v EN nepoužívá"
        );
    }

    #[test]
    fn german_pipeline() {
        assert_eq!(
            transform(
                "Er sagte \"Hallo\" - z. B. am 1. 1. 2026, siehe Nr. 5.",
                Lang::De
            ),
            "Er sagte „Hallo“\u{a0}– z.\u{a0}B. am 1.\u{a0}1.\u{a0}2026, siehe Nr.\u{a0}5."
        );
        assert_eq!(
            transform(
                "Die Last beträgt 10 kN, geht's um 1 000 000 Zyklen...",
                Lang::De
            ),
            "Die Last beträgt 10\u{a0}kN, geht\u{2019}s um 1\u{202f}000\u{202f}000 Zyklen\u{2026}"
        );
    }
}

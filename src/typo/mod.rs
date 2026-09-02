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
/// párování), tisícové skupiny až po jednotkách (obě pracují s číslicemi).
pub fn transform(text: &str, lang: Lang) -> String {
    match lang {
        Lang::Cs => {
            let s = quotes::czech_quotes(text);
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
            misc::ellipsis(&s)
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
    fn english_gets_english_quotes_only() {
        assert_eq!(
            transform("He said \"hello\" to k friend...", Lang::En),
            "He said \u{201c}hello\u{201d} to k friend\u{2026}"
        );
    }
}

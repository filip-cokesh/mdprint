//! Měkké rozdělovníky (U+00AD) přes crate `hyphenation` s embedovanými vzory.
//! Pravidla: slovo ≥ 6 znaků, ≥ 3 znaky před i za zlomem, nadpisy se nedělí.
//! CSS `hyphens: manual` pak zlomy aktivuje jen při zalomení řádku.

use anyhow::{Context, Result};
use comrak::nodes::{AstNode, NodeValue};
use hyphenation::{Hyphenator as _, Language, Load, Standard};

use crate::cli::Lang;

const SOFT_HYPHEN: char = '\u{ad}';
const MIN_WORD_CHARS: usize = 6;
const MIN_MARGIN_CHARS: usize = 3;

pub struct Hyphenator {
    dict: Standard,
}

impl Hyphenator {
    pub fn new(lang: Lang) -> Result<Self> {
        let language = match lang {
            Lang::Cs => Language::Czech,
            Lang::En => Language::EnglishUS,
            Lang::De => Language::German1996,
        };
        let dict = Standard::from_embedded(language)
            .with_context(|| format!("embedovaný slovník dělení pro {language:?} chybí"))?;
        Ok(Hyphenator { dict })
    }

    /// Vloží měkké rozdělovníky do textových uzlů; podstromy nadpisů vynechá.
    pub fn apply<'a>(&self, root: &'a AstNode<'a>) {
        for node in root.descendants() {
            let in_heading = node
                .ancestors()
                .any(|a| matches!(a.data.borrow().value, NodeValue::Heading(_)));
            if in_heading {
                continue;
            }
            let mut data = node.data.borrow_mut();
            if let NodeValue::Text(t) = &mut data.value {
                let hyphenated = self.hyphenate_text(t);
                if hyphenated != *t.as_ref() {
                    *t = hyphenated.into();
                }
            }
        }
    }

    /// Rozdělí text na běhy písmen (slova) a ostatek; slova doplní o U+00AD.
    fn hyphenate_text(&self, text: &str) -> String {
        let mut out = String::with_capacity(text.len() + 8);
        let mut word = String::new();
        for c in text.chars() {
            if c.is_alphabetic() || c == SOFT_HYPHEN {
                word.push(c);
            } else {
                if !word.is_empty() {
                    self.push_word(&mut out, &word);
                    word.clear();
                }
                out.push(c);
            }
        }
        if !word.is_empty() {
            self.push_word(&mut out, &word);
        }
        out
    }

    fn push_word(&self, out: &mut String, word: &str) {
        let char_count = word.chars().count();
        if char_count < MIN_WORD_CHARS || word.contains(SOFT_HYPHEN) {
            out.push_str(word);
            return;
        }
        let hyphenated = self.dict.hyphenate(word);
        let mut last = 0usize;
        for &brk in &hyphenated.breaks {
            let before = word[..brk].chars().count();
            if before < MIN_MARGIN_CHARS || char_count - before < MIN_MARGIN_CHARS {
                continue;
            }
            out.push_str(&word[last..brk]);
            out.push(SOFT_HYPHEN);
            last = brk;
        }
        out.push_str(&word[last..]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;
    use comrak::Arena;

    const SHY: &str = "\u{ad}";

    fn cs() -> Hyphenator {
        Hyphenator::new(Lang::Cs).unwrap()
    }

    #[test]
    fn hyphenates_long_czech_words() {
        let h = cs();
        let out = h.hyphenate_text("nejneobhospodařovávatelnějšímu");
        assert!(out.contains(SHY), "{out:?}");
        // žádný zlom blíž než 3 znaky od okrajů
        let first = out.find(SHY).unwrap();
        let last = out.rfind(SHY).unwrap();
        assert!(out[..first].chars().count() >= 3);
        assert!(out[last..].chars().filter(|c| *c != '\u{ad}').count() >= 3);
    }

    #[test]
    fn short_words_untouched() {
        let h = cs();
        for w in ["dům", "lesu", "pěti", "slovo"] {
            assert_eq!(h.hyphenate_text(w), w, "slovo {w:?} se nemá dělit");
        }
    }

    #[test]
    fn interpunction_and_numbers_preserved() {
        let h = cs();
        let out = h.hyphenate_text("Nosník (železobetonový), rozpětí 12 m.");
        let stripped: String = out.chars().filter(|c| *c != '\u{ad}').collect();
        assert_eq!(stripped, "Nosník (železobetonový), rozpětí 12 m.");
        assert!(out.contains(SHY), "dlouhé slovo se má dělit: {out:?}");
    }

    #[test]
    fn existing_soft_hyphens_respected() {
        let h = cs();
        let manual = "že\u{ad}lezobeton";
        assert_eq!(h.hyphenate_text(manual), manual);
    }

    #[test]
    fn headings_not_hyphenated_but_paragraphs_yes() {
        let arena = Arena::new();
        let root = parse::parse(
            &arena,
            "## Nejneobhospodařovávatelnějšímu\n\nNejneobhospodařovávatelnějšímu pozemku.\n",
            &parse::options(),
        );
        cs().apply(root);
        let mut heading_text = String::new();
        let mut para_text = String::new();
        for node in root.descendants() {
            if let NodeValue::Text(t) = &node.data.borrow().value {
                let in_heading = node
                    .ancestors()
                    .any(|a| matches!(a.data.borrow().value, NodeValue::Heading(_)));
                if in_heading {
                    heading_text.push_str(t);
                } else {
                    para_text.push_str(t);
                }
            }
        }
        assert!(!heading_text.contains(SHY), "{heading_text:?}");
        assert!(para_text.contains(SHY), "{para_text:?}");
    }

    #[test]
    fn english_dictionary_loads_and_hyphenates() {
        let h = Hyphenator::new(Lang::En).unwrap();
        let out = h.hyphenate_text("hyphenation");
        assert!(out.contains(SHY), "{out:?}");
    }
}

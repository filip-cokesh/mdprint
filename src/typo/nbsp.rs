//! Nezlomitelné mezery (U+00A0) dle ČSN 01 6910.

use std::sync::LazyLock;

use fancy_regex::Regex;

const NBSP: &str = "\u{a0}";

fn re(pattern: &str) -> Regex {
    Regex::new(pattern).expect("vadný regex")
}

fn replace_all(re: &Regex, text: &str, rep: &str) -> String {
    re.replace_all(text, rep).into_owned()
}

/// Za jednopísmennými předložkami a spojkami k, s, v, z, o, u, a, i
/// (i velkými) se řádek nesmí zlomit.
pub fn single_letter_prepositions(text: &str) -> String {
    static RE: LazyLock<Regex> =
        LazyLock::new(|| re(r"(?<=^|[\s\u{a0}(\[{„‚>])([ksvzouaiKSVZOUAI]) "));
    // opakovaně kvůli řetězení „a v i…“ (lookbehind na předchozí náhradu)
    let once = replace_all(&RE, text, &format!("$1{NBSP}"));
    replace_all(&RE, &once, &format!("$1{NBSP}"))
}

/// Mezi číslem a jednotkou (`10 kg`, `25 °C`, `100 %`).
pub fn number_unit(text: &str) -> String {
    static RE: LazyLock<Regex> = LazyLock::new(|| {
        re(
            r"(?<=\d) (°C|°F|%|‰|kg|mg|g|t|km|mm|cm|dm|m|ha|hl|ml|dl|l|ms|s|min|h|hod|kHz|MHz|GHz|Hz|kPa|MPa|GPa|Pa|kN|MN|N|kJ|MJ|J|kWh|kW|MW|GW|W|mV|kV|V|mA|A|Kč|€|K|mil\.|mld\.|tis\.)(?=$|[\s\u{a0}.,;:!?)\]}])",
        )
    });
    replace_all(&RE, text, &format!("{NBSP}$1"))
}

/// Za zkratkami (`č.`, `str.`, `tj.`, `např.` …) následovanými dalším výrazem.
pub fn abbreviations(text: &str) -> String {
    static RE: LazyLock<Regex> = LazyLock::new(|| {
        re(
            r"(?<=^|[\s\u{a0}(\[{„‚])(č|čís|str|s|tj|tzn|tzv|např|popř|příp|resp|mj|př|kap|obr|tab|roč|odst|písm)\. ",
        )
    });
    replace_all(&RE, text, &format!("$1.{NBSP}"))
}

/// Mezi iniciálou a dalším (velkým) jménem: `J. Novák`, `J. K. Tyl`.
pub fn initials(text: &str) -> String {
    static RE: LazyLock<Regex> = LazyLock::new(|| {
        re(r"(?<=^|[\s\u{a0}(„‚])([A-ZÁČĎÉĚÍŇÓŘŠŤÚŮÝŽ])\. (?=[A-ZÁČĎÉĚÍŇÓŘŠŤÚŮÝŽ])")
    });
    let once = replace_all(&RE, text, &format!("$1.{NBSP}"));
    replace_all(&RE, &once, &format!("$1.{NBSP}"))
}

/// Uvnitř numerického data: `1. 1. 2026` (jazykově neutrální).
pub fn dates_numeric(text: &str) -> String {
    static NUMERIC: LazyLock<Regex> = LazyLock::new(|| re(r"\b(\d{1,2})\. (\d{1,2})\. (\d{4})\b"));
    replace_all(&NUMERIC, text, &format!("$1.{NBSP}$2.{NBSP}$3"))
}

/// Česká data: numerická + `1. ledna` s názvem měsíce.
pub fn dates(text: &str) -> String {
    static MONTH: LazyLock<Regex> = LazyLock::new(|| {
        re(
            r"\b(\d{1,2})\. (ledna|února|března|dubna|května|června|července|srpna|září|října|listopadu|prosince)\b",
        )
    });
    replace_all(&MONTH, &dates_numeric(text), &format!("$1.{NBSP}$2"))
}

/// Německá data: numerická + `1. Januar` s názvem měsíce.
pub fn dates_de(text: &str) -> String {
    static MONTH: LazyLock<Regex> = LazyLock::new(|| {
        re(
            r"\b(\d{1,2})\. (Januar|Februar|März|April|Mai|Juni|Juli|August|September|Oktober|November|Dezember)\b",
        )
    });
    replace_all(&MONTH, &dates_numeric(text), &format!("$1.{NBSP}$2"))
}

/// Německé zkratky dle DIN 5008: mezera UVNITŘ vícedílných zkratek je
/// nezlomitelná (`z. B.`, `d. h.`, `i. d. R.` …); zkratky před číslem
/// (`Nr. 5`, `S. 12`, `Abb. 3`) se váží na číslo.
pub fn german_abbreviations(text: &str) -> String {
    static MULTI: LazyLock<Regex> = LazyLock::new(|| {
        re(
            r"(?<=^|[\s\u{a0}(\[{„‚])(z\. B\.|d\. h\.|u\. a\.|u\. U\.|o\. Ä\.|u\. Ä\.|z\. T\.|s\. o\.|s\. u\.|i\. d\. R\.|i\. A\.|n\. Chr\.|v\. Chr\.)",
        )
    });
    static BEFORE_NUM: LazyLock<Regex> = LazyLock::new(|| re(r"\b(Nr|S|Abb|Tab|Kap|Bd)\. (?=\d)"));
    // ruční průchod: v každém nálezu nahradit mezery za NBSP
    let mut bound = String::with_capacity(text.len());
    let mut last = 0;
    for m in MULTI.find_iter(text).flatten() {
        bound.push_str(&text[last..m.start()]);
        bound.push_str(&m.as_str().replace(' ', NBSP));
        last = m.end();
    }
    bound.push_str(&text[last..]);
    replace_all(&BEFORE_NUM, &bound, &format!("$1.{NBSP}"))
}

/// `§ 12` → `§ 12` s nezlomitelnou mezerou.
pub fn paragraph_sign(text: &str) -> String {
    static RE: LazyLock<Regex> = LazyLock::new(|| re(r"§ (?=\d)"));
    replace_all(&RE, text, &format!("§{NBSP}"))
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
    fn prepositions() {
        table(
            single_letter_prepositions,
            &[
                ("k lesu", "k\u{a0}lesu"),
                ("Jdu v zimě o berlích", "Jdu v\u{a0}zimě o\u{a0}berlích"),
                ("A i U s K", "A\u{a0}i\u{a0}U\u{a0}s\u{a0}K"),
                ("(v závorce)", "(v\u{a0}závorce)"),
                ("„u citátu“", "„u\u{a0}citátu“"),
                // písmeno uvnitř slova se nedotýká
                ("kva s tančí", "kva s\u{a0}tančí"),
                ("kilo vody", "kilo vody"),
            ],
        );
    }

    #[test]
    fn units() {
        table(
            number_unit,
            &[
                ("10 kg", "10\u{a0}kg"),
                ("25 °C", "25\u{a0}°C"),
                ("100 %", "100\u{a0}%"),
                ("210 GPa,", "210\u{a0}GPa,"),
                ("5 mm a 3 kN.", "5\u{a0}mm a 3\u{a0}kN."),
                ("1500 Kč", "1500\u{a0}Kč"),
                // jednotka nesmí „ukousnout“ začátek slova
                ("10 metrů", "10 metrů"),
                ("2 ml mléka", "2\u{a0}ml mléka"),
            ],
        );
    }

    #[test]
    fn abbrevs() {
        table(
            abbreviations,
            &[
                ("č. 12", "č.\u{a0}12"),
                ("str. 45", "str.\u{a0}45"),
                ("tj. celek", "tj.\u{a0}celek"),
                ("např. tady", "např.\u{a0}tady"),
                ("viz obr. 3 a tab. 2", "viz obr.\u{a0}3 a tab.\u{a0}2"),
                // konec věty tečkou nezkratkou se nemění
                ("dům. Pak", "dům. Pak"),
            ],
        );
    }

    #[test]
    fn initials_and_dates() {
        table(
            initials,
            &[
                ("J. Novák", "J.\u{a0}Novák"),
                ("J. K. Tyl", "J.\u{a0}K.\u{a0}Tyl"),
                ("konec věty. Další", "konec věty. Další"),
            ],
        );
        table(
            dates,
            &[
                ("1. 1. 2026", "1.\u{a0}1.\u{a0}2026"),
                ("narozen 28. 10. 1918.", "narozen 28.\u{a0}10.\u{a0}1918."),
                ("5. července", "5.\u{a0}července"),
                // řadová číslovka bez data se nemění
                ("5. kapitola", "5. kapitola"),
            ],
        );
    }

    #[test]
    fn german_dates_and_abbrevs() {
        table(
            dates_de,
            &[
                ("am 1. 1. 2026", "am 1.\u{a0}1.\u{a0}2026"),
                ("am 3. Oktober 1990", "am 3.\u{a0}Oktober 1990"),
                ("das 5. Kapitel", "das 5. Kapitel"),
            ],
        );
        table(
            german_abbreviations,
            &[
                ("z. B. hier", "z.\u{a0}B. hier"),
                ("d. h. sofort", "d.\u{a0}h. sofort"),
                ("i. d. R. gilt", "i.\u{a0}d.\u{a0}R. gilt"),
                ("siehe Nr. 5 und S. 12", "siehe Nr.\u{a0}5 und S.\u{a0}12"),
                ("vgl. Abb. 3 und Tab. 2", "vgl. Abb.\u{a0}3 und Tab.\u{a0}2"),
                // konec věty tečkou se nemění
                ("Haus. Der", "Haus. Der"),
            ],
        );
    }

    #[test]
    fn paragraph() {
        table(
            paragraph_sign,
            &[("§ 12", "§\u{a0}12"), ("dle § 1058", "dle §\u{a0}1058")],
        );
    }
}

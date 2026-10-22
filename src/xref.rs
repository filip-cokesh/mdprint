//! Křížové odkazy: labely rovnic, obrázků a tabulek → čísla a odkazy
//! `\ref{…}` / `\eqref{…}` / `\autoref{…}` v textu, položky pro seznam
//! obrázků a tabulek (`--lof`, `--lot`).
//!
//! Běží hned po parsování, **před** typografií a dělením slov — ty přepisují
//! `Text` uzly (nezlomitelné mezery, U+00AD) a labely by rozbily. Syntaxe
//! je LaTeXová: `\label{eq:x}` kdekoli v blokové rovnici, `{#fig:x}` za
//! obrázkem (comrak `link_attributes`), odstavec `Tabulka: popis {#tbl:x}`
//! bezprostředně před tabulkou. KaTeX `\label`/`\ref` nezná, proto je
//! odstraňuje a čísla vkládá tento modul (`\tag{N}`).

use std::cell::RefCell;
use std::collections::HashMap;

use anyhow::{Result, anyhow, bail};
use comrak::Arena;
use comrak::nodes::{Ast, AstNode, NodeValue};
use serde::Deserialize;

use crate::cli::Lang;
use crate::parse::{collect_text, is_figure_image};

/// Které blokové rovnice číslovat (`[equations] numbering`).
#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum EqNumbering {
    /// Jen rovnice s `\label{…}` (analogie `equation` vs. `equation*`).
    #[default]
    Labeled,
    /// Všechny `$$…$$`; opt-out `\notag` / `\nonumber`.
    All,
    /// Žádná; labely se odstraní, odkaz na rovnici je chyba.
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Equation,
    Figure,
    Table,
}

/// Cíl odkazu: druh, id kotvy v HTML, zobrazované číslo (u rovnice
/// s vlastním `\tag` jeho text) a řádek zdroje.
#[derive(Debug, Clone)]
pub struct Target {
    pub kind: Kind,
    pub id: String,
    pub number: String,
    pub line: usize,
}

/// Lokalizované prefixy pro `\autoref` a popisky.
#[derive(Debug, Clone)]
pub struct Prefixes {
    pub figure: String,
    pub table: String,
    pub lang: Lang,
}

impl Prefixes {
    fn equation(&self) -> &'static str {
        match self.lang {
            Lang::Cs => "rovnice",
            Lang::En => "Eq.",
            Lang::De => "Gl.",
        }
    }
}

/// Výchozí prefix popisku tabulky podle jazyka.
pub fn default_table_prefix(lang: Lang) -> &'static str {
    match lang {
        Lang::Cs => "Tab.",
        Lang::En => "Table",
        Lang::De => "Tab.",
    }
}

/// Klíčová slova popisku tabulky (přijímána v každém jazyce).
const TABLE_KEYWORDS: [&str; 3] = ["tabulka:", "table:", "tabelle:"];

/// Registr cílů v pořadí dokumentu.
#[derive(Debug, Default)]
pub struct Xref {
    targets: Vec<Target>,
    by_id: HashMap<String, usize>,
}

impl Xref {
    pub fn get(&self, id: &str) -> Option<&Target> {
        self.by_id.get(id).map(|&i| &self.targets[i])
    }

    pub fn targets(&self) -> &[Target] {
        &self.targets
    }

    fn register(&mut self, target: Target) -> Result<()> {
        if let Some(prev) = self.get(&target.id) {
            bail!(
                "duplicitní label „{}“ (řádek {}, poprvé na řádku {})",
                target.id,
                target.line,
                prev.line
            );
        }
        self.by_id.insert(target.id.clone(), self.targets.len());
        self.targets.push(target);
        Ok(())
    }
}

/// Label: písmeno, pak písmena, číslice a `: _ . -` (bezpečné jako HTML id
/// i fragment URL, bez escapování).
fn valid_label(s: &str) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, ':' | '_' | '.' | '-'))
}

fn check_label(label: &str, line: usize) -> Result<()> {
    if !valid_label(label) {
        bail!(
            "neplatný label „{label}“ (řádek {line}): povolena písmena, číslice a : _ . -, začíná písmenem"
        );
    }
    Ok(())
}

/// Najde `\name{arg}` v `s`; vrací (začátek, konec za `}`, arg).
fn find_command(s: &str, name: &str) -> Option<(usize, usize, String)> {
    let needle = format!("\\{name}{{");
    let start = s.find(&needle)?;
    let arg_start = start + needle.len();
    let rel_end = s[arg_start..].find('}')?;
    let arg = s[arg_start..arg_start + rel_end].trim().to_string();
    Some((start, arg_start + rel_end + 1, arg))
}

/// Odstraní všechny výskyty příkazu bez argumentu (`\notag`, `\nonumber`),
/// jen pokud za ním nenásleduje písmeno (aby `\notagx` zůstalo).
fn strip_bare_command(s: &str, name: &str) -> (String, bool) {
    let needle = format!("\\{name}");
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    let mut found = false;
    while let Some(pos) = rest.find(&needle) {
        let after = &rest[pos + needle.len()..];
        let is_word_end = !after.starts_with(|c: char| c.is_ascii_alphabetic());
        out.push_str(&rest[..pos]);
        if is_word_end {
            found = true;
        } else {
            out.push_str(&needle);
        }
        rest = after;
    }
    out.push_str(rest);
    (out, found)
}

/// Průchod 1: labely → čísla. Upraví AST: odstraní `\label`, vloží `\tag`,
/// doplní `attrs.id` rovnicím, obrázkům a tabulkám, označí popisky tabulek.
pub fn collect<'a>(root: &'a AstNode<'a>, numbering: EqNumbering) -> Result<Xref> {
    let mut xref = Xref::default();
    let (mut equations, mut figures, mut tables) = (0usize, 0usize, 0usize);

    for node in root.descendants() {
        let line = node.data.borrow().sourcepos.start.line;
        let is_table = matches!(node.data.borrow().value, NodeValue::Table(_));
        if is_table {
            if let Some(target) = collect_table(node, &mut tables, line)? {
                xref.register(target)?;
            }
            continue;
        }

        let mut data = node.data.borrow_mut();
        match &mut data.value {
            NodeValue::Math(m) => {
                let label = match find_command(&m.literal, "label") {
                    Some((start, end, label)) => {
                        m.literal.replace_range(start..end, "");
                        Some(label)
                    }
                    None => None,
                };
                if !m.display_math {
                    if let Some(label) = label {
                        bail!(
                            "\\label{{{label}}} v řádkové matematice (řádek {line}): číslovat lze jen blokové rovnice $$…$$"
                        );
                    }
                    continue;
                }
                if let Some(label) = &label {
                    check_label(label, line)?;
                }
                let (stripped, notag) = strip_bare_command(&m.literal, "notag");
                let (stripped, nonumber) = strip_bare_command(&stripped, "nonumber");
                let notag = notag || nonumber;
                m.literal = stripped;
                let own_tag = find_command(&m.literal, "tag")
                    .or_else(|| find_command(&m.literal, "tag*"))
                    .map(|(_, _, t)| t);

                let numbered = match numbering {
                    EqNumbering::Labeled => label.is_some(),
                    EqNumbering::All => !notag,
                    EqNumbering::None => false,
                };
                if label.is_some() && notag {
                    bail!("rovnice má \\label i \\notag (řádek {line})");
                }
                if label.is_some() && numbering == EqNumbering::None {
                    bail!("\\label v rovnici (řádek {line}), ale [equations] numbering = \"none\"");
                }
                if !numbered {
                    continue;
                }
                let number = match own_tag {
                    Some(t) => t,
                    None => {
                        equations += 1;
                        let _ = std::fmt::Write::write_fmt(
                            &mut m.literal,
                            format_args!(" \\tag{{{equations}}}"),
                        );
                        equations.to_string()
                    }
                };
                let id = label.unwrap_or_else(|| format!("eq-{number}"));
                set_id(&mut data, &id);
                xref.register(Target {
                    kind: Kind::Equation,
                    id,
                    number,
                    line,
                })?;
            }
            NodeValue::Image(_) => {
                drop(data);
                if !is_figure_image(node) {
                    continue;
                }
                figures += 1;
                let mut data = node.data.borrow_mut();
                let label = data.attrs.as_ref().and_then(|a| a.id.clone());
                if let Some(label) = &label {
                    check_label(label, line)?;
                }
                let id = label.unwrap_or_else(|| format!("fig-{figures}"));
                set_id(&mut data, &id);
                xref.register(Target {
                    kind: Kind::Figure,
                    id,
                    number: figures.to_string(),
                    line,
                })?;
            }
            _ => {}
        }
    }
    Ok(xref)
}

fn set_id(data: &mut Ast, id: &str) {
    data.attrs.get_or_insert_with(Default::default).id = Some(id.to_string());
}

/// Má uzel třídu `table-caption` (popisek tabulky označený průchodem 1)?
pub fn is_table_caption<'a>(node: &'a AstNode<'a>) -> bool {
    node.data
        .borrow()
        .attrs
        .as_ref()
        .is_some_and(|a| a.classes.iter().any(|c| c == "table-caption"))
}

/// Id uzlu z atributů (label nebo doplněné `fig-N` / `tab-N` / `eq-N`).
pub fn node_id<'a>(node: &'a AstNode<'a>) -> Option<String> {
    node.data.borrow().attrs.as_ref().and_then(|a| a.id.clone())
}

/// Tabulka s popiskem: předchozí sourozenec je odstavec začínající
/// `Tabulka:` / `Table:` / `Tabelle:`. Prefix a koncové `{#label}` se
/// z textu odstraní, odstavec dostane třídu `table-caption`, tabulka id.
fn collect_table<'a>(
    table: &'a AstNode<'a>,
    tables: &mut usize,
    line: usize,
) -> Result<Option<Target>> {
    let Some(caption) = table.previous_sibling() else {
        return Ok(None);
    };
    if !matches!(caption.data.borrow().value, NodeValue::Paragraph) {
        return Ok(None);
    }
    let Some(first) = caption.first_child() else {
        return Ok(None);
    };
    // prefix musí být v prvním textovém uzlu (ne v kurzívě apod.)
    let prefix_len = {
        let data = first.data.borrow();
        let NodeValue::Text(t) = &data.value else {
            return Ok(None);
        };
        let lower = t.to_lowercase();
        match TABLE_KEYWORDS.iter().find(|k| lower.starts_with(*k)) {
            Some(k) => k.len(),
            None => return Ok(None),
        }
    };
    {
        let mut data = first.data.borrow_mut();
        if let NodeValue::Text(t) = &mut data.value {
            let rest = t[prefix_len..].trim_start().to_string();
            *t = rest.into();
        }
    }

    // {#label} na konci posledního textového uzlu
    let mut label = None;
    if let Some(last) = caption.last_child() {
        let mut data = last.data.borrow_mut();
        if let NodeValue::Text(t) = &mut data.value {
            let trimmed = t.trim_end();
            if let Some(open) = trimmed.rfind("{#")
                && trimmed.ends_with('}')
            {
                let candidate = &trimmed[open + 2..trimmed.len() - 1];
                if !candidate.contains('}') {
                    label = Some(candidate.to_string());
                    let kept = trimmed[..open].trim_end().to_string();
                    *t = kept.into();
                }
            }
        }
    }
    if let Some(label) = &label {
        check_label(label, line)?;
    }
    if collect_text(caption).trim().is_empty() {
        bail!("prázdný popisek tabulky (řádek {line})");
    }

    *tables += 1;
    let id = label.unwrap_or_else(|| format!("tab-{tables}"));
    caption
        .data
        .borrow_mut()
        .attrs
        .get_or_insert_with(Default::default)
        .classes
        .push("table-caption".into());
    set_id(&mut table.data.borrow_mut(), &id);
    Ok(Some(Target {
        kind: Kind::Table,
        id,
        number: tables.to_string(),
        line,
    }))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum RefKind {
    Ref,
    EqRef,
    AutoRef,
}

const REF_COMMANDS: [(&str, RefKind); 3] = [
    ("\\autoref{", RefKind::AutoRef),
    ("\\eqref{", RefKind::EqRef),
    ("\\ref{", RefKind::Ref),
];

/// Jeden textový uzel rozdělený na prostý text a odkazy.
#[derive(Debug, PartialEq)]
enum Segment {
    Text(String),
    Ref(RefKind, String),
}

fn split_refs(text: &str) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut plain = String::new();
    let mut rest = text;
    'outer: while let Some(pos) = rest.find('\\') {
        for (cmd, kind) in REF_COMMANDS {
            if let Some(after) = rest[pos..].strip_prefix(cmd)
                && let Some(close) = after.find('}')
            {
                plain.push_str(&rest[..pos]);
                if !plain.is_empty() {
                    segments.push(Segment::Text(std::mem::take(&mut plain)));
                }
                segments.push(Segment::Ref(kind, after[..close].trim().to_string()));
                rest = &after[close + 1..];
                continue 'outer;
            }
        }
        plain.push_str(&rest[..=pos]);
        rest = &rest[pos + 1..];
    }
    plain.push_str(rest);
    if !plain.is_empty() || segments.is_empty() {
        segments.push(Segment::Text(plain));
    }
    segments
}

fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

/// Text odkazu podle druhu příkazu a cíle.
fn ref_text(kind: RefKind, target: &Target, prefixes: &Prefixes, line: usize) -> Result<String> {
    let n = &target.number;
    Ok(match (kind, target.kind) {
        (RefKind::Ref, _) => n.clone(),
        (RefKind::EqRef, Kind::Equation) => format!("({n})"),
        (RefKind::EqRef, _) => bail!(
            "\\eqref{{{}}} (řádek {line}) odkazuje na {} — \\eqref je jen pro rovnice, použijte \\ref nebo \\autoref",
            target.id,
            match target.kind {
                Kind::Figure => "obrázek",
                Kind::Table => "tabulku",
                Kind::Equation => unreachable!(),
            }
        ),
        (RefKind::AutoRef, Kind::Equation) => format!("{}\u{a0}({n})", prefixes.equation()),
        (RefKind::AutoRef, Kind::Figure) => format!("{}\u{a0}{n}", prefixes.figure),
        (RefKind::AutoRef, Kind::Table) => format!("{}\u{a0}{n}", prefixes.table),
    })
}

/// Průchod 2: `\ref{…}` / `\eqref{…}` / `\autoref{…}` v textu → `<a class="xref">`.
/// Textový uzel se rozdělí na text + HtmlInline + text, aby typografie
/// a dělení slov dál viděly jen prostý text.
pub fn resolve<'a>(
    arena: &'a Arena<'a>,
    root: &'a AstNode<'a>,
    xref: &Xref,
    prefixes: &Prefixes,
) -> Result<()> {
    let candidates: Vec<&AstNode<'a>> = root
        .descendants()
        .filter(|n| matches!(&n.data.borrow().value, NodeValue::Text(t) if t.contains("\\")))
        .collect();

    for node in candidates {
        let (text, sourcepos) = {
            let data = node.data.borrow();
            let NodeValue::Text(t) = &data.value else {
                continue;
            };
            (t.to_string(), data.sourcepos)
        };
        let segments = split_refs(&text);
        if segments.len() == 1 && matches!(segments[0], Segment::Text(_)) {
            continue;
        }
        let line = sourcepos.start.line;
        let mut values = Vec::with_capacity(segments.len());
        for seg in segments {
            values.push(match seg {
                Segment::Text(t) => NodeValue::Text(t.into()),
                Segment::Ref(kind, label) => {
                    let target = xref.get(&label).ok_or_else(|| {
                        anyhow!("neznámý odkaz \\…{{{label}}} (řádek {line}): takový label v dokumentu není")
                    })?;
                    let text = ref_text(kind, target, prefixes, line)?;
                    NodeValue::HtmlInline(
                        format!(
                            "<a class=\"xref\" href=\"#{}\">{}</a>",
                            target.id,
                            escape_html(&text)
                        ),
                    )
                }
            });
        }
        let mut values = values.into_iter();
        node.data.borrow_mut().value = values.next().expect("aspoň jeden segment");
        let mut last = node;
        for value in values {
            let new = arena.alloc(AstNode::new(RefCell::new(Ast::new_with_sourcepos(
                value, sourcepos,
            ))));
            last.insert_after(new);
            last = new;
        }
    }
    Ok(())
}

/// Položka seznamu obrázků / tabulek.
#[derive(Debug, Clone)]
pub struct ListEntry {
    /// „Obr. 1“ / „Tab. 2“ (bez dvojtečky)
    pub label: String,
    pub id: String,
    pub text: String,
}

/// Položky pro `--lof` / `--lot`. Volat **po** typografii a dělení slov,
/// aby text popisků odpovídal sazbě v dokumentu.
pub fn list_entries<'a>(root: &'a AstNode<'a>, kind: Kind, prefix: &str) -> Vec<ListEntry> {
    let mut entries = Vec::new();
    for node in root.descendants() {
        let (caption_node, is_match) = match kind {
            Kind::Figure => (node, is_figure_image(node)),
            Kind::Table => {
                let is_table = matches!(node.data.borrow().value, NodeValue::Table(_));
                match node.previous_sibling() {
                    Some(prev) if is_table && is_table_caption(prev) => (prev, true),
                    _ => (node, false),
                }
            }
            Kind::Equation => (node, false),
        };
        if !is_match {
            continue;
        }
        let Some(id) = node_id(node) else {
            continue;
        };
        let number = entries.len() + 1;
        entries.push(ListEntry {
            label: format!("{prefix}\u{a0}{number}"),
            id,
            text: collect_text(caption_node),
        });
    }
    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;

    fn run(md: &str, numbering: EqNumbering, lang: Lang) -> Result<(String, Vec<Target>)> {
        let opts = parse::options();
        let arena = Arena::new();
        let root = parse::parse(&arena, md, &opts);
        let xref = collect(root, numbering)?;
        let prefixes = Prefixes {
            figure: match lang {
                Lang::Cs => "Obr.",
                Lang::En => "Fig.",
                Lang::De => "Abb.",
            }
            .into(),
            table: default_table_prefix(lang).into(),
            lang,
        };
        resolve(&arena, root, &xref, &prefixes)?;
        let mut html = String::new();
        comrak::html::format_document(root, &opts, &mut html)?;
        // literal rovnic ještě není vyrenderovaný — vypíšeme ho syrově
        let mut math = Vec::new();
        for node in root.descendants() {
            if let NodeValue::Math(m) = &node.data.borrow().value {
                math.push(m.literal.clone());
            }
        }
        Ok((
            format!("{html}\n---\n{}", math.join("\n")),
            xref.targets().to_vec(),
        ))
    }

    #[test]
    fn labeled_equation_gets_tag_and_id() {
        let md = "$$ a = b \\label{eq:ab} $$\n\nViz \\eqref{eq:ab} a \\ref{eq:ab}, tj. \\autoref{eq:ab}.\n";
        let (out, targets) = run(md, EqNumbering::Labeled, Lang::Cs).unwrap();
        assert!(out.contains(" a = b   \\tag{1}"), "{out}");
        assert!(!out.contains("\\label"), "{out}");
        assert!(
            out.contains(r##"<a class="xref" href="#eq:ab">(1)</a>"##),
            "{out}"
        );
        assert!(
            out.contains(r##"<a class="xref" href="#eq:ab">1</a>"##),
            "{out}"
        );
        assert!(
            out.contains("<a class=\"xref\" href=\"#eq:ab\">rovnice\u{a0}(1)</a>"),
            "{out}"
        );
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].id, "eq:ab");
        assert_eq!(targets[0].kind, Kind::Equation);
    }

    #[test]
    fn unlabeled_equation_not_numbered_in_labeled_mode() {
        let md = "$$ a = b $$\n\n$$ c = d \\label{eq:cd} $$\n";
        let (out, targets) = run(md, EqNumbering::Labeled, Lang::Cs).unwrap();
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].number, "1", "číslo se počítá jen u číslovaných");
        assert!(out.contains(" a = b \n"), "první bez tagu: {out}");
    }

    #[test]
    fn all_mode_numbers_everything_except_notag() {
        let md = "$$ a $$\n\n$$ b \\notag $$\n\n$$ c \\nonumber $$\n\n$$ d \\label{eq:d} $$\n";
        let (out, targets) = run(md, EqNumbering::All, Lang::En).unwrap();
        let ids: Vec<&str> = targets.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, ["eq-1", "eq:d"]);
        assert_eq!(targets[1].number, "2");
        assert!(
            !out.contains("\\notag") && !out.contains("\\nonumber"),
            "{out}"
        );
    }

    #[test]
    fn own_tag_is_kept_and_referenced() {
        let md = "$$ x \\tag{A} \\label{eq:x} $$\n\n\\eqref{eq:x}\n";
        let (out, targets) = run(md, EqNumbering::Labeled, Lang::Cs).unwrap();
        assert_eq!(targets[0].number, "A");
        let math = out.split("\n---\n").nth(1).unwrap();
        assert_eq!(math.matches("\\tag").count(), 1, "žádný druhý tag: {out}");
        assert!(out.contains(">(A)</a>"), "{out}");
    }

    #[test]
    fn none_mode_rejects_label() {
        let err = run("$$ x \\label{eq:x} $$\n", EqNumbering::None, Lang::Cs).unwrap_err();
        assert!(format!("{err}").contains("none"), "{err}");
        // bez labelů projde a nic nečísluje
        let (_, targets) = run("$$ x $$\n", EqNumbering::None, Lang::Cs).unwrap();
        assert!(targets.is_empty());
    }

    #[test]
    fn errors_with_line_numbers() {
        let err = run(
            "text\n\n$a \\label{eq:i}$\n",
            EqNumbering::Labeled,
            Lang::Cs,
        )
        .unwrap_err();
        assert!(format!("{err}").contains("řádek 3"), "{err}");

        let err = run(
            "$$ a \\label{eq:a} $$\n\n$$ b \\label{eq:a} $$\n",
            EqNumbering::Labeled,
            Lang::Cs,
        )
        .unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("duplicitní") && msg.contains("řádek 3"),
            "{msg}"
        );

        let err = run("Viz \\ref{eq:neni}.\n", EqNumbering::Labeled, Lang::Cs).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("neznámý") && msg.contains("řádek 1"), "{msg}");

        let err = run("$$ a \\label{eq a} $$\n", EqNumbering::Labeled, Lang::Cs).unwrap_err();
        assert!(format!("{err}").contains("neplatný label"), "{err}");

        let err = run(
            "$$ a \\label{eq:a} \\notag $$\n",
            EqNumbering::All,
            Lang::Cs,
        )
        .unwrap_err();
        assert!(format!("{err}").contains("\\notag"), "{err}");
    }

    #[test]
    fn figures_get_ids_from_attributes_or_sequence() {
        let md = "![Nosník](a.png){#fig:nosnik}\n\n![Graf](b.png)\n\nViz \\autoref{fig:nosnik} a \\ref{fig-2}.\n";
        let (out, targets) = run(md, EqNumbering::Labeled, Lang::De).unwrap();
        let ids: Vec<&str> = targets.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, ["fig:nosnik", "fig-2"]);
        assert!(
            out.contains(r##"<a class="xref" href="#fig:nosnik">Abb.&nbsp;1</a>"##)
                || out.contains("<a class=\"xref\" href=\"#fig:nosnik\">Abb.\u{a0}1</a>"),
            "{out}"
        );
        assert!(out.contains(r##"href="#fig-2">2</a>"##), "{out}");
        // atribut se v Markdownu „spotřeboval“ — není v textu
        assert!(!out.contains("{#fig:nosnik}"), "{out}");
    }

    #[test]
    fn inline_image_is_not_a_figure() {
        let md = "Text ![ikona](i.png) dál.\n";
        let (_, targets) = run(md, EqNumbering::Labeled, Lang::Cs).unwrap();
        assert!(targets.is_empty());
    }

    #[test]
    fn table_caption_detected_stripped_and_labeled() {
        let md = "Tabulka: Vstupní *data* {#tbl:data}\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\nViz \\autoref{tbl:data}, \\ref{tbl:data}.\n";
        let (out, targets) = run(md, EqNumbering::Labeled, Lang::Cs).unwrap();
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].id, "tbl:data");
        assert_eq!(targets[0].kind, Kind::Table);
        assert!(!out.contains("Tabulka:"), "prefix má zmizet: {out}");
        assert!(!out.contains("{#tbl:data}"), "label má zmizet: {out}");
        assert!(out.contains("Vstupní <em>data</em>"), "{out}");
        assert!(
            out.contains("<a class=\"xref\" href=\"#tbl:data\">Tab.\u{a0}1</a>"),
            "{out}"
        );
    }

    #[test]
    fn table_keywords_any_language_and_unnumbered_without_caption() {
        for kw in ["Table:", "TABELLE:", "tabulka:"] {
            let md = format!("{kw} Popis\n\n| a |\n|---|\n| 1 |\n");
            let (_, targets) = run(&md, EqNumbering::Labeled, Lang::En).unwrap();
            assert_eq!(targets.len(), 1, "{kw}");
            assert_eq!(targets[0].id, "tab-1");
        }
        let md = "Následující tabulka shrnuje:\n\n| a |\n|---|\n| 1 |\n";
        let (out, targets) = run(md, EqNumbering::Labeled, Lang::Cs).unwrap();
        assert!(targets.is_empty());
        assert!(out.contains("Následující tabulka shrnuje:"));
    }

    #[test]
    fn caption_paragraph_without_table_is_untouched() {
        let md = "Tabulka: jen text\n\nDalší odstavec.\n";
        let (out, targets) = run(md, EqNumbering::Labeled, Lang::Cs).unwrap();
        assert!(targets.is_empty());
        assert!(out.contains("Tabulka: jen text"));
    }

    #[test]
    fn eqref_on_figure_is_error() {
        let md = "![a](a.png){#fig:a}\n\n\\eqref{fig:a}\n";
        let err = run(md, EqNumbering::Labeled, Lang::Cs).unwrap_err();
        assert!(
            format!("{err}").contains("\\eqref je jen pro rovnice"),
            "{err}"
        );
    }

    #[test]
    fn autoref_texts_per_language() {
        let md = "$$ a \\label{eq:a} $$\n\n![f](f.png){#fig:f}\n\nTable: t {#tbl:t}\n\n| a |\n|---|\n| 1 |\n\n\\autoref{eq:a} \\autoref{fig:f} \\autoref{tbl:t}\n";
        let expect = [
            (Lang::Cs, ["rovnice\u{a0}(1)", "Obr.\u{a0}1", "Tab.\u{a0}1"]),
            (Lang::En, ["Eq.\u{a0}(1)", "Fig.\u{a0}1", "Table\u{a0}1"]),
            (Lang::De, ["Gl.\u{a0}(1)", "Abb.\u{a0}1", "Tab.\u{a0}1"]),
        ];
        for (lang, texts) in expect {
            let (out, _) = run(md, EqNumbering::Labeled, lang).unwrap();
            for t in texts {
                assert!(
                    out.contains(&format!(">{t}</a>")),
                    "{lang:?}: {t} chybí v {out}"
                );
            }
        }
    }

    #[test]
    fn split_refs_keeps_other_backslashes() {
        let segs = split_refs("a \\b \\ref{x} c\\");
        assert_eq!(
            segs,
            vec![
                Segment::Text("a \\b ".into()),
                Segment::Ref(RefKind::Ref, "x".into()),
                Segment::Text(" c\\".into()),
            ]
        );
        assert_eq!(
            split_refs("bez odkazu"),
            vec![Segment::Text("bez odkazu".into())]
        );
    }

    #[test]
    fn list_entries_follow_document_order() {
        let md = "![Druhý? Ne, první](a.png){#fig:a}\n\nTabulka: První tabulka\n\n| a |\n|---|\n| 1 |\n\n![Druhý](b.png)\n";
        let opts = parse::options();
        let arena = Arena::new();
        let root = parse::parse(&arena, md, &opts);
        collect(root, EqNumbering::Labeled).unwrap();
        let figs = list_entries(root, Kind::Figure, "Obr.");
        assert_eq!(figs.len(), 2);
        assert_eq!(
            (figs[0].label.as_str(), figs[0].id.as_str()),
            ("Obr.\u{a0}1", "fig:a")
        );
        assert_eq!(figs[1].text, "Druhý");
        let tabs = list_entries(root, Kind::Table, "Tab.");
        assert_eq!(tabs.len(), 1);
        assert_eq!(tabs[0].text, "První tabulka");
        assert_eq!(tabs[0].id, "tab-1");
    }
}

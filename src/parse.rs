use anyhow::{Context, Result};
use comrak::nodes::{AstNode, NodeValue};
use comrak::{Arena, Options, parse_document};
use serde::Deserialize;

use crate::cli::Lang;

/// Volby comraku pro celý pipeline (parse i render sdílí jednu instanci).
pub fn options() -> Options<'static> {
    let mut opts = Options::default();
    let ext = &mut opts.extension;
    ext.table = true;
    ext.strikethrough = true;
    ext.footnotes = true;
    ext.description_lists = true;
    ext.math_dollars = true;
    ext.front_matter_delimiter = Some("---".into());
    // Prázdný prefix: id kotvy = slug nadpisu.
    ext.header_id_prefix = Some(String::new());
    // Nutné pro průchod HTML, které samy generují moduly math a highlight;
    // vedlejší efekt: projde i raw HTML autora (u osobního nástroje záměr).
    opts.render.r#unsafe = true;
    opts
}

pub fn parse<'a>(arena: &'a Arena<'a>, source: &str, opts: &Options) -> &'a AstNode<'a> {
    parse_document(arena, source, opts)
}

/// YAML hlavička dokumentu; všechna pole volitelná.
#[derive(Deserialize, Default, Debug)]
pub struct FrontMatter {
    pub title: Option<String>,
    pub author: Option<String>,
    #[serde(default, deserialize_with = "flexible_string")]
    pub date: Option<String>,
    pub lang: Option<Lang>,
    /// Verze dokumentu (`version: "1.2"`); YAML bez uvozovek by ji přečetl
    /// jako číslo (a `1.20` zkrátil na `1.2`), proto bere číslo i řetězec.
    #[serde(default, deserialize_with = "flexible_string")]
    pub version: Option<String>,
    /// Šablona: "default", nebo cesta ke složce packu (relativně ke vstupu).
    pub template: Option<String>,
}

/// YAML skalár (string i číslo) → `Option<String>`.
fn flexible_string<'de, D>(de: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value: Option<serde_yaml::Value> = serde::Deserialize::deserialize(de)?;
    Ok(value.and_then(|v| match v {
        serde_yaml::Value::String(s) => Some(s),
        serde_yaml::Value::Number(n) => Some(n.to_string()),
        serde_yaml::Value::Null => None,
        other => serde_yaml::to_string(&other)
            .ok()
            .map(|s| s.trim().to_string()),
    }))
}

/// Vytáhne front matter z AST (comrak ho nechává jako první uzel včetně `---` oddělovačů).
pub fn front_matter<'a>(root: &'a AstNode<'a>) -> Result<Option<FrontMatter>> {
    for node in root.children() {
        if let NodeValue::FrontMatter(raw) = &node.data.borrow().value {
            let yaml = strip_delimiters(raw);
            let fm: FrontMatter = serde_yaml::from_str(yaml).context("chybný YAML front matter")?;
            return Ok(Some(fm));
        }
    }
    Ok(None)
}

fn strip_delimiters(raw: &str) -> &str {
    let trimmed = raw.trim();
    let body = trimmed.strip_prefix("---").unwrap_or(trimmed);
    let body = body.strip_suffix("---").unwrap_or(body);
    body.trim()
}

/// Titulek dokumentu: front matter `title`, jinak text prvního H1, jinak jméno souboru.
pub fn document_title<'a>(
    front_matter: Option<&FrontMatter>,
    root: &'a AstNode<'a>,
    file_stem: &str,
) -> String {
    if let Some(title) = front_matter.and_then(|fm| fm.title.clone()) {
        return title;
    }
    for node in root.descendants() {
        if let NodeValue::Heading(h) = node.data.borrow().value
            && h.level == 1
        {
            return collect_text(node);
        }
    }
    file_stem.to_string()
}

/// Prostý text uzlu a jeho potomků (pro titulky, popisky).
pub fn collect_text<'a>(node: &'a AstNode<'a>) -> String {
    let mut out = String::new();
    for d in node.descendants() {
        match &d.data.borrow().value {
            NodeValue::Text(t) => out.push_str(t),
            NodeValue::Code(c) => out.push_str(&c.literal),
            NodeValue::SoftBreak | NodeValue::LineBreak => out.push(' '),
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn front_matter_parsed() {
        let src = "---\ntitle: Pokus\nauthor: S. H.\ndate: 2026-09-02\nlang: en\nversion: \"1.2\"\n---\n\n# Ahoj\n";
        let arena = Arena::new();
        let root = parse(&arena, src, &options());
        let fm = front_matter(root).unwrap().expect("front matter chybí");
        assert_eq!(fm.title.as_deref(), Some("Pokus"));
        assert_eq!(fm.author.as_deref(), Some("S. H."));
        assert_eq!(fm.date.as_deref(), Some("2026-09-02"));
        assert_eq!(fm.lang, Some(Lang::En));
        assert_eq!(fm.version.as_deref(), Some("1.2"));
    }

    #[test]
    fn version_accepts_bare_yaml_number() {
        let src = "---\ntitle: T\nversion: 1.2\n---\n\ntext\n";
        let arena = Arena::new();
        let root = parse(&arena, src, &options());
        let fm = front_matter(root).unwrap().unwrap();
        assert_eq!(fm.version.as_deref(), Some("1.2"));
    }

    #[test]
    fn missing_front_matter_falls_back_to_h1() {
        let arena = Arena::new();
        let root = parse(&arena, "# První *nadpis*\n\ntext\n", &options());
        assert!(front_matter(root).unwrap().is_none());
        assert_eq!(document_title(None, root, "soubor"), "První nadpis");
    }

    #[test]
    fn no_h1_falls_back_to_file_stem() {
        let arena = Arena::new();
        let root = parse(&arena, "odstavec bez nadpisu\n", &options());
        assert_eq!(document_title(None, root, "soubor"), "soubor");
    }
}

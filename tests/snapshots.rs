//! Snapshot testy celého HTML výstupu (insta). Blok <style> se před snapshotem
//! nahrazuje značkou — base64 fontů by snapshot učinilo nečitelným.

use std::fs;
use std::path::Path;

use mdprint::cli::Cli;

fn build_fixture(name: &str, md_file: &str, template: Option<String>) -> String {
    let src = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    let dir = tempfile::tempdir().unwrap();
    copy_tree(&src, dir.path());

    let cli = Cli {
        input: dir.path().join(md_file),
        output: None,
        lang: None,
        toc: false,
        fetch: false,
        config: None,
        template,
    };
    let out = mdprint::run(&cli).unwrap();
    let html = fs::read_to_string(out).unwrap();
    normalize(&strip_style(&html))
}

fn copy_tree(src: &Path, dst: &Path) {
    for entry in fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let target = dst.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            fs::create_dir_all(&target).unwrap();
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

fn strip_style(html: &str) -> String {
    let start = html.find("<style>").expect("chybí <style>");
    let end = html.find("</style>").expect("chybí </style>") + "</style>".len();
    format!("{}<style>…</style>{}", &html[..start], &html[end..])
}

/// katex-rs vypisuje atributy tagů a vlastnosti ve `style` v nedeterministickém
/// pořadí (mezi běhy se liší). Pro stabilní snapshot obojí seřadíme.
fn normalize(html: &str) -> String {
    let style_re = regex::Regex::new(r#"style="([^"]*)""#).unwrap();
    let html = style_re.replace_all(html, |c: &regex::Captures| {
        let mut parts: Vec<&str> = c[1]
            .split(';')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        parts.sort_unstable();
        format!("style=\"{};\"", parts.join(";"))
    });
    let tag_re =
        regex::Regex::new(r#"<([a-zA-Z][\w-]*)((?:\s+[\w:-]+="[^"]*")+)(\s*/?)>"#).unwrap();
    let attr_re = regex::Regex::new(r#"\s+[\w:-]+="[^"]*""#).unwrap();
    tag_re
        .replace_all(&html, |c: &regex::Captures| {
            let mut attrs: Vec<&str> = attr_re.find_iter(&c[2]).map(|m| m.as_str()).collect();
            attrs.sort_unstable();
            format!("<{}{}{}>", &c[1], attrs.concat(), &c[3])
        })
        .into_owned()
}

#[test]
fn akademicky_dokument() {
    let html = build_fixture("akademicky", "pruzkum-pruhybu.md", None);
    insta::assert_snapshot!("akademicky", html);
}

#[test]
fn akademicky_dokument_pres_pack() {
    let pack = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/pack-demo")
        .to_string_lossy()
        .into_owned();
    let html = build_fixture("akademicky", "pruzkum-pruhybu.md", Some(pack));
    assert!(html.contains("brand-header"));
    assert!(html.contains("Demo s.r.o."));
    // data URI log jsou obrovské — pro snapshot je zkrátíme
    let re = regex::Regex::new(r"data:image/png;base64,[A-Za-z0-9+/=]+").unwrap();
    let html = re
        .replace_all(&html, "data:image/png;base64,…")
        .into_owned();
    insta::assert_snapshot!("akademicky-pack", html);
}

#[test]
fn plna_stranka_ma_styly_a_fonty() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/akademicky");
    let dir = tempfile::tempdir().unwrap();
    copy_tree(&src, dir.path());
    let cli = Cli {
        input: dir.path().join("pruzkum-pruhybu.md"),
        output: None,
        lang: None,
        toc: false,
        fetch: false,
        config: None,
        template: None,
    };
    let out = mdprint::run(&cli).unwrap();
    let html = fs::read_to_string(out).unwrap();

    // 10 řezů dokumentových fontů + 20 KaTeX fontů
    assert_eq!(html.matches("@font-face").count(), 30);
    assert!(html.contains("@page"));
    assert!(html.contains("@media print"));
    assert!(html.contains("<html lang=\"cs\">"));
    // fonty jsou skutečně inlinované (WOFF2 magic v base64)
    assert!(html.contains("base64,d09G"));
    // matematika vyrenderovaná při buildu, kód se statickými spany
    assert!(html.contains("class=\"katex\""));
    assert!(html.contains("katex-display"));
    assert!(html.contains("<span class=\"line\">"));
    // tmavý režim: jen pro screen, s CSS-only přepínačem; tisk ho skrývá
    assert!(html.contains("@media screen and (prefers-color-scheme: dark)"));
    assert!(html.contains("id=\"theme-invert\""));
    assert!(html.contains("label.theme-switch { display: none; }"));
    assert!(
        !html.contains("<script"),
        "výstup nesmí obsahovat JavaScript"
    );
}

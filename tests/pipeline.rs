use std::fs;
use std::path::{Path, PathBuf};

use mdprint::cli::{Cli, Lang};

fn cli(input: PathBuf) -> Cli {
    Cli {
        input,
        output: None,
        lang: None,
        toc: false,
        fetch: false,
        config: None,
        template: None,
    }
}

fn img_files(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n != ".manifest.json")
        .collect();
    names.sort();
    names
}

#[test]
fn copies_dedups_and_rewrites_images() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join("a")).unwrap();
    fs::create_dir_all(root.join("b")).unwrap();
    // stejný název ve dvou složkách, různý obsah
    fs::write(root.join("a/schéma.png"), b"obsah-A").unwrap();
    fs::write(root.join("b/schéma.png"), b"obsah-B").unwrap();
    // identický obsah pod jiným názvem → dedup
    fs::write(root.join("kopie.png"), b"obsah-A").unwrap();

    let md = "\
# Test\n\n\
![první](a/schéma.png)\n\n\
![druhý](b/sch%C3%A9ma.png)\n\n\
![týž obsah](kopie.png)\n\n\
![opakování](a\\schéma.png)\n\n\
![vzdálený](https://example.com/x.png)\n";
    let input = root.join("dokument.md");
    fs::write(&input, md).unwrap();

    let out = mdprint::run(&cli(input)).unwrap();
    assert_eq!(out, root.join("dokument.html"));

    let img_dir = root.join("img_dokument");
    let names = img_files(&img_dir);
    // obsah-A (jednou, ač 3 odkazy) + obsah-B = 2 soubory
    assert_eq!(names.len(), 2, "očekávány 2 soubory, jsou: {names:?}");
    assert!(
        names
            .iter()
            .all(|n| n.starts_with("schema-") || n.starts_with("kopie-"))
    );

    let html = fs::read_to_string(&out).unwrap();
    for name in &names {
        assert!(
            html.contains(&format!("img_dokument/{name}")),
            "HTML neodkazuje na {name}"
        );
    }
    assert!(
        html.contains("https://example.com/x.png"),
        "vzdálená URL má zůstat beze změny"
    );

    let manifest = fs::read_to_string(img_dir.join(".manifest.json")).unwrap();
    for name in &names {
        assert!(manifest.contains(name));
    }
}

#[test]
fn rebuild_sweeps_only_orphans_from_manifest() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::write(root.join("x.png"), b"X").unwrap();
    fs::write(root.join("y.png"), b"Y").unwrap();
    let input = root.join("doc.md");

    fs::write(&input, "![x](x.png)\n\n![y](y.png)\n").unwrap();
    mdprint::run(&cli(input.clone())).unwrap();
    let img_dir = root.join("img_doc");
    assert_eq!(img_files(&img_dir).len(), 2);

    // cizí soubor, kterého se úklid nesmí dotknout
    fs::write(img_dir.join("cizi.txt"), "nech mě").unwrap();

    // druhý build už y.png nepoužívá → jeho kopie zmizí, cizí soubor zůstane
    fs::write(&input, "![x](x.png)\n").unwrap();
    mdprint::run(&cli(input)).unwrap();
    let names = img_files(&img_dir);
    assert_eq!(names.len(), 2, "čekán 1 obrázek + cizi.txt, je: {names:?}");
    assert!(names.contains(&"cizi.txt".to_string()));
    assert!(names.iter().any(|n| n.starts_with("x-")));
    assert!(
        !names.iter().any(|n| n.starts_with("y-")),
        "osiřelý y-*.png měl být smazán"
    );
}

#[test]
fn missing_image_fails_with_line_number() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("doc.md");
    fs::write(&input, "# Nadpis\n\ntext\n\n![chybí](neni.png)\n").unwrap();

    let err = mdprint::run(&cli(input)).unwrap_err();
    let msg = format!("{err:#}");
    assert!(msg.contains("neni.png"), "chybí cesta v chybě: {msg}");
    assert!(msg.contains("řádek 5"), "chybí číslo řádku v chybě: {msg}");
}

#[test]
fn toc_links_match_heading_ids() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("doc.md");
    fs::write(
        &input,
        "# Úvod\n\ntext\n\n## Česká sekce\n\ntext\n\n### Detail\n\ntext\n\n## Česká sekce\n\ntext\n\n#### Hluboký nadpis mimo TOC\n",
    )
    .unwrap();

    let mut c = cli(input);
    c.toc = true;
    let out = mdprint::run(&c).unwrap();
    let html = fs::read_to_string(&out).unwrap();

    assert!(html.contains("<nav class=\"toc\">"));
    assert!(html.contains(">Obsah</h2>"));
    // odkazy míří na skutečná id nadpisů (vč. deduplikace „-1“)
    for anchor in ["#úvod", "#česká-sekce", "#detail", "#česká-sekce-1"] {
        assert!(
            html.contains(&format!("<a href=\"{anchor}\">")),
            "chybí TOC odkaz {anchor}"
        );
        let id = format!("id=\"{}\"", &anchor[1..]);
        assert!(html.contains(&id), "chybí kotva {id}");
    }
    // h4 do TOC nepatří
    assert!(!html.contains("#hluboký-nadpis-mimo-toc\">"));

    // bez --toc žádný obsah
    let mut c2 = cli(dir.path().join("doc.md"));
    c2.toc = false;
    let out = mdprint::run(&c2).unwrap();
    assert!(
        !fs::read_to_string(&out)
            .unwrap()
            .contains("<nav class=\"toc\">")
    );
}

#[test]
fn fetch_fails_offline_and_uses_disk_cache() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("doc.md");
    // .invalid doména dle RFC 2606 nikdy neexistuje — bez cache musí běh selhat
    let url = "https://example.invalid/obrazky/graf.png?v=2";
    fs::write(&input, format!("![vzdálený]({url})\n")).unwrap();

    let mut c = cli(input.clone());
    c.fetch = true;
    let err = format!("{:#}", mdprint::run(&c).unwrap_err());
    assert!(err.contains("stažení obrázku selhalo"), "{err}");
    assert!(err.contains("řádek 1"), "{err}");

    // s předvyplněnou cache (deterministické jméno z URL) běh projde offline
    let name = mdprint::images::remote_target_name(url);
    assert!(
        name.starts_with("graf-") && name.ends_with(".png"),
        "{name}"
    );
    let img_dir = dir.path().join("img_doc");
    fs::create_dir_all(&img_dir).unwrap();
    fs::write(img_dir.join(&name), b"png-data").unwrap();

    let mut c = cli(input);
    c.fetch = true;
    let out = mdprint::run(&c).unwrap();
    let html = fs::read_to_string(&out).unwrap();
    assert!(
        html.contains(&format!("img_doc/{name}")),
        "{name} v HTML chybí"
    );
    // bez --fetch zůstává URL beze změny
    let out = mdprint::run(&cli(dir.path().join("doc.md"))).unwrap();
    assert!(
        fs::read_to_string(&out)
            .unwrap()
            .contains("example.invalid")
    );
}

#[test]
fn headerless_table_hides_empty_thead() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("doc.md");
    fs::write(&input, "| | |\n|---|---|\n| a | b |\n| c | d |\n").unwrap();

    let out = mdprint::run(&cli(input)).unwrap();
    let html = fs::read_to_string(&out).unwrap();
    // comrak generuje skutečně prázdné buňky — na tom stojí CSS :empty selektor…
    assert!(html.contains("<th></th>"), "prázdné th už nejsou prázdné");
    // …které prázdný thead skryje (druhá linka nad tabulkou, opakování v tisku)
    assert!(html.contains("thead:not(:has(th:not(:empty))) { display: none; }"));
}

#[test]
fn version_appears_in_byline() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("doc.md");
    fs::write(
        &input,
        "---\ntitle: T\nauthor: A. B.\ndate: 2026-09-02\nversion: \"1.2\"\n---\n\ntext\n",
    )
    .unwrap();
    let out = mdprint::run(&cli(input.clone())).unwrap();
    let html = fs::read_to_string(&out).unwrap();
    assert!(
        html.contains("A. B. · 2.\u{a0}9.\u{a0}2026 · verze 1.2"),
        "byline nesedí"
    );

    // jen verze bez autora a data
    fs::write(&input, "---\ntitle: T\nversion: 3\n---\n\ntext\n").unwrap();
    let out = mdprint::run(&cli(input)).unwrap();
    assert!(fs::read_to_string(&out).unwrap().contains(">verze 3</p>"));
}

fn demo_pack() -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/pack-demo")
        .to_string_lossy()
        .into_owned()
}

#[test]
fn template_pack_selection_and_company_footer() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("doc.md");
    fs::write(&input, "# Dokument\n\ntext\n").unwrap();

    // výchozí šablona: žádný branding
    let out = mdprint::run(&cli(input.clone())).unwrap();
    let html = fs::read_to_string(&out).unwrap();
    assert!(!html.contains("brand-header"));

    // --template <pack>: hlavička s názvem firmy z pack.toml, obě loga, patička, font
    let mut c = cli(input.clone());
    c.template = Some(demo_pack());
    let out = mdprint::run(&c).unwrap();
    let html = fs::read_to_string(&out).unwrap();
    assert!(html.contains("class=\"brand-header\""));
    assert!(html.contains("<span class=\"brand-name\">Demo s.r.o.</span>"));
    assert!(html.contains("<span class=\"brand-footer-name\">Demo s.r.o.</span>"));
    assert!(
        html.contains("demo.example"),
        "web z pack.toml patří do patičky"
    );
    assert!(
        html.contains("class=\"brand-logo-light\"") && html.contains("class=\"brand-logo-dark\"")
    );
    assert_eq!(html.matches("data:image/png;base64,").count(), 2);
    assert!(html.contains("'Demo Mono'"), "font packu má být inlinovaný");
    // regrese bugu „dvě loga najednou": skrytí musí přebít `.brand-header img`
    assert!(html.contains(".brand-header .brand-logo-dark { display: none; }"));

    // mdprint.toml: template (relativně k tomlu) + [company] přebíjí pack defaulty
    let pack_toml_escaped = demo_pack().replace('\\', "/");
    fs::write(
        dir.path().join("mdprint.toml"),
        format!(
            "template = \"{pack_toml_escaped}\"\n\n[company]\nname = \"Toml a.s.\"\naddress = \"Brno\"\nico = \"12345678\"\n"
        ),
    )
    .unwrap();
    let out = mdprint::run(&cli(input.clone())).unwrap();
    let html = fs::read_to_string(&out).unwrap();
    assert!(html.contains("brand-header"), "toml má zapnout pack");
    assert!(
        html.contains("Toml a.s."),
        "toml [company] přebíjí pack.toml"
    );
    assert!(
        html.contains("Brno · IČ 12345678 · demo.example"),
        "merge pack+toml"
    );

    // CLI „default" přebíjí toml
    let mut c = cli(input.clone());
    c.template = Some("default".into());
    let out = mdprint::run(&c).unwrap();
    assert!(!fs::read_to_string(&out).unwrap().contains("brand-header"));

    // front matter přebíjí toml
    fs::write(dir.path().join("mdprint.toml"), "template = \"default\"\n").unwrap();
    fs::write(
        &input,
        format!("---\ntitle: T\ntemplate: \"{pack_toml_escaped}\"\n---\n\ntext\n"),
    )
    .unwrap();
    let out = mdprint::run(&cli(input)).unwrap();
    assert!(fs::read_to_string(&out).unwrap().contains("brand-header"));
}

#[test]
fn missing_pack_fails_with_path() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("doc.md");
    fs::write(&input, "text\n").unwrap();
    let mut c = cli(input);
    c.template = Some(dir.path().join("neni-pack").to_string_lossy().into_owned());
    let err = format!("{:#}", mdprint::run(&c).unwrap_err());
    assert!(err.contains("složka šablony nenalezena"), "{err}");
    assert!(err.contains("neni-pack"), "{err}");
}

#[test]
fn lang_priority_cli_over_front_matter() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("doc.md");
    fs::write(&input, "---\ntitle: T\nlang: en\n---\n\n# T\n").unwrap();

    // front matter vyhrává nad výchozím cs
    let out = mdprint::run(&cli(input.clone())).unwrap();
    assert!(
        fs::read_to_string(&out)
            .unwrap()
            .contains("<html lang=\"en\">")
    );

    // CLI přebíjí front matter
    let mut c = cli(input);
    c.lang = Some(Lang::Cs);
    let out = mdprint::run(&c).unwrap();
    assert!(
        fs::read_to_string(&out)
            .unwrap()
            .contains("<html lang=\"cs\">")
    );
}

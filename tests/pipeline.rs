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
        ..Default::default()
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

#[test]
fn toc_depth_and_numbering_from_toml() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("kniha.md");
    fs::write(
        &input,
        "# 1 Úvod\n\ntext\n\n## 1.1 Cíle\n\ntext\n\n### 1.1.1 Detail\n\ntext\n",
    )
    .unwrap();
    let mut c = cli(input.clone());
    c.toc = true;

    // výchozí: číslovaný obsah do úrovně 3
    let html = fs::read_to_string(mdprint::run(&c).unwrap()).unwrap();
    assert!(html.contains("<nav class=\"toc\">"));
    assert!(html.contains("<a href=\"#111-detail\">"));

    fs::write(
        dir.path().join("mdprint.toml"),
        "[toc]\nnumbering = false\ndepth = 2\n",
    )
    .unwrap();
    let html = fs::read_to_string(mdprint::run(&c).unwrap()).unwrap();
    assert!(html.contains("<nav class=\"toc toc-plain\">"));
    assert!(html.contains("<a href=\"#11-cíle\">"));
    assert!(!html.contains("<a href=\"#111-detail\">"));

    // mimo rozsah = čitelná chyba
    fs::write(dir.path().join("mdprint.toml"), "[toc]\ndepth = 0\n").unwrap();
    let err = mdprint::run(&c).unwrap_err();
    assert!(err.to_string().contains("mimo rozsah 1–6"), "{err}");
}

#[test]
fn math_size_from_toml_and_range_validation() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("m.md");
    fs::write(&input, "Text $E = mc^2$.\n\n$$\\frac{a}{b}$$\n").unwrap();

    // default bez tomlu
    let out = mdprint::run(&cli(input.clone())).unwrap();
    let html = fs::read_to_string(&out).unwrap();
    assert!(html.contains("--math-inline:0.95em;--math-display:1em;"));

    // [math] z mdprint.toml
    fs::write(
        dir.path().join("mdprint.toml"),
        "[math]\ninline = 1.1\ndisplay = 1.3\n",
    )
    .unwrap();
    let out = mdprint::run(&cli(input.clone())).unwrap();
    let html = fs::read_to_string(&out).unwrap();
    assert!(html.contains("--math-inline:1.1em;--math-display:1.3em;"));

    // mimo rozsah = čitelná chyba
    fs::write(dir.path().join("mdprint.toml"), "[math]\ndisplay = 3.0\n").unwrap();
    let err = mdprint::run(&cli(input)).unwrap_err();
    assert!(err.to_string().contains("mimo rozsah 0.5–2.0"), "{err}");
}

#[test]
fn math_wide_tiers_scale_with_configured_size() {
    // vzorec pod polovinou rozpočtu (21–42 em) při defaultu → bez tieru,
    // s display = 2.0 → fyzicky 2× širší → tier
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("w.md");
    // pozor: holé "math-wide" je vždy přítomné v inlinovaném print.css —
    // testovat se musí třída na vzorci
    let tex = "a_1 + a_2 + a_3 + a_4 + a_5 + a_6 + a_7 + a_8 + a_9 + b_1 + b_2 + b_3";
    let tier = "class=\"katex-display math-wide";
    fs::write(&input, format!("$${tex}$$\n")).unwrap();

    let out = mdprint::run(&cli(input.clone())).unwrap();
    let html = fs::read_to_string(&out).unwrap();
    assert!(!html.contains(tier), "default nesmí mít tier");

    fs::write(dir.path().join("mdprint.toml"), "[math]\ndisplay = 2.0\n").unwrap();
    let out = mdprint::run(&cli(input)).unwrap();
    let html = fs::read_to_string(&out).unwrap();
    assert!(html.contains(tier), "zvětšená math musí do tieru");
}

/// Dokument přes několik stran (dlouhé odstavce pod každým nadpisem).
fn long_doc() -> String {
    let para = "Odstavec s dostatečně dlouhým textem, aby se dokument roztáhl \
                přes více stran při tisku na formát A4. "
        .repeat(30);
    let mut md = String::from("---\ntitle: Test stránkování\n---\n\n");
    for (i, h) in ["Úvod", "Metoda", "Výsledky", "Diskuse", "Závěr"]
        .iter()
        .enumerate()
    {
        md.push_str(&format!("# {h}\n\n"));
        for _ in 0..=i {
            md.push_str(&format!("{para}\n\n"));
        }
    }
    md
}

#[test]
fn page_numbers_and_font_size_from_config() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("doc.md");
    fs::write(&input, "# A\n\ntext\n").unwrap();

    // výchozí: číslování zapnuté, radio normal
    let html = fs::read_to_string(mdprint::run(&cli(input.clone())).unwrap()).unwrap();
    assert!(html.contains("@bottom-right"), "chybí číslo stránky");
    assert!(html.contains(r#"id="fs-normal" checked>"#));

    // [print] v tomlu: bez čísel, velké písmo
    fs::write(
        dir.path().join("mdprint.toml"),
        "[print]\npage_numbers = false\nfont_size = \"large\"\n",
    )
    .unwrap();
    let html = fs::read_to_string(mdprint::run(&cli(input.clone())).unwrap()).unwrap();
    assert!(!html.contains("@bottom-right"));
    assert!(html.contains(r#"id="fs-large" checked>"#));
    assert!(html.contains(r#"id="fs-normal">"#));

    // CLI přebije toml
    let mut c = cli(input);
    c.font_size = Some(mdprint::cli::FontSize::Small);
    let html = fs::read_to_string(mdprint::run(&c).unwrap()).unwrap();
    assert!(html.contains(r#"id="fs-small" checked>"#));
}

#[test]
fn toc_pages_without_browser_warns_and_falls_back() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("doc.md");
    fs::write(&input, "# Úvod\n\ntext\n\n# Závěr\n\ntext\n").unwrap();

    let mut c = cli(input);
    c.toc_pages = true; // zapíná i obsah
    c.browser = Some(dir.path().join("neni-chrome.exe"));
    let out = mdprint::run_full(&c).unwrap();
    assert_eq!(out.warnings.len(), 1, "{:?}", out.warnings);
    assert!(out.warnings[0].contains("prohlížeč nenalezen"));
    let html = fs::read_to_string(&out.html).unwrap();
    assert!(html.contains("<nav class=\"toc\">"), "obsah bez čísel");
    assert!(!html.contains(r#"class="toc-page""#));
    // žádné sondy po sobě nenechá
    assert_eq!(
        fs::read_dir(dir.path()).unwrap().count(),
        2,
        "doc.md + doc.html"
    );

    // --pdf bez prohlížeče je chyba (výslovný požadavek)
    c.pdf = Some(mdprint::cli::PdfMode::One);
    let err = format!("{:#}", mdprint::run_full(&c).unwrap_err());
    assert!(err.contains("--pdf vyžaduje Chrome nebo Edge"), "{err}");
}

/// Dokument s popsanou tabulkou a figurou pro testy seznamů (1×1 PNG).
fn doc_with_table_and_figure(dir: &Path) -> PathBuf {
    const PNG_1X1: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F,
        0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0xF8,
        0xFF, 0xFF, 0x3F, 0x00, 0x05, 0xFE, 0x02, 0xFE, 0xA7, 0x35, 0x81, 0x84, 0x00, 0x00, 0x00,
        0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];
    fs::write(dir.join("bod.png"), PNG_1X1).unwrap();
    let input = dir.join("doc.md");
    let mut md = long_doc();
    md.push_str(
        "\n# Přílohy\n\nViz \\autoref{fig:bod}, \\autoref{tbl:data} a rovnici \\eqref{eq:x}.\n\n\
         $$ x = 1 \\label{eq:x} $$\n\n\
         ![Jeden bod](bod.png){#fig:bod}\n\n\
         Tabulka: Vstupní data {#tbl:data}\n\n| a | b |\n|---|---|\n| 1 | 2 |\n",
    );
    fs::write(&input, md).unwrap();
    input
}

#[test]
fn lists_without_browser_fall_back_with_single_warning() {
    let dir = tempfile::tempdir().unwrap();
    let input = doc_with_table_and_figure(dir.path());

    let mut c = cli(input);
    c.toc_pages = true;
    c.lof = true;
    c.lot = true;
    c.browser = Some(dir.path().join("neni-chrome.exe"));
    let out = mdprint::run_full(&c).unwrap();
    assert_eq!(
        out.warnings.len(),
        1,
        "jedno měření = jedno varování: {:?}",
        out.warnings
    );
    let html = fs::read_to_string(&out.html).unwrap();
    assert!(
        html.contains("<nav class=\"toc lof\">"),
        "seznam obrázků bez čísel"
    );
    assert!(
        html.contains("<nav class=\"toc lot\">"),
        "seznam tabulek bez čísel"
    );
    assert!(!html.contains(r#"class="toc-page""#));
    // kotvy sedí: id na figure/table/rovnici = href v seznamech a odkazech
    assert!(html.contains("<figure id=\"fig:bod\">"));
    assert!(html.contains("<figure class=\"table\" id=\"tbl:data\">"));
    assert!(html.contains("<span id=\"eq:x\" class=\"katex-display\""));
    assert!(html.contains("<a href=\"#fig:bod\"><span class=\"toc-num\">Obr.\u{a0}1</span>"));
    assert!(html.contains("<a href=\"#tbl:data\"><span class=\"toc-num\">Tab.\u{a0}1</span>"));
    assert!(html.contains("<a class=\"xref\" href=\"#eq:x\">(1)</a>"));
    // seznamy jsou až za tělem dokumentu
    let body_end = html.find("Vstupní data").unwrap();
    assert!(html.find("<nav class=\"toc lof\">").unwrap() > body_end);
    // žádné sondy po sobě nenechá
    assert_eq!(
        fs::read_dir(dir.path()).unwrap().count(),
        4,
        "doc.md, bod.png, doc.html, img_doc"
    );
}

#[test]
fn lists_from_front_matter_and_toml() {
    let dir = tempfile::tempdir().unwrap();
    let input = doc_with_table_and_figure(dir.path());

    // bez voleb: žádné seznamy, ale id a odkazy ano
    let html = fs::read_to_string(mdprint::run(&cli(input.clone())).unwrap()).unwrap();
    assert!(!html.contains("toc lof") && !html.contains("toc lot"));
    assert!(html.contains("<figure id=\"fig:bod\">"));

    // [figures] list / [tables] list v tomlu + prefix tabulky
    fs::write(
        dir.path().join("mdprint.toml"),
        "[figures]\nlist = true\n[tables]\nlist = true\nprefix = \"Tabulka\"\n",
    )
    .unwrap();
    let html = fs::read_to_string(mdprint::run(&cli(input.clone())).unwrap()).unwrap();
    assert!(html.contains("toc lof") && html.contains("toc lot"));
    assert!(html.contains("<span class=\"tabnum\">Tabulka 1:</span>"));
    assert!(html.contains("<a class=\"xref\" href=\"#tbl:data\">Tabulka\u{a0}1</a>"));

    // front matter přebije toml
    let md = fs::read_to_string(&input).unwrap();
    fs::write(&input, format!("---\nlof: false\nlot: false\n---\n\n{md}")).unwrap();
    let html = fs::read_to_string(mdprint::run(&cli(input)).unwrap()).unwrap();
    assert!(!html.contains("toc lof") && !html.contains("toc lot"));
}

#[test]
fn equation_numbering_all_mode_from_toml() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("doc.md");
    fs::write(&input, "$$ a $$\n\n$$ b \\notag $$\n\n$$ c $$\n").unwrap();
    fs::write(
        dir.path().join("mdprint.toml"),
        "[equations]\nnumbering = \"all\"\n",
    )
    .unwrap();
    let html = fs::read_to_string(mdprint::run(&cli(input.clone())).unwrap()).unwrap();
    assert!(html.contains("<span id=\"eq-1\" class=\"katex-display\""));
    assert!(html.contains("<span id=\"eq-2\" class=\"katex-display\""));
    let body = html.split("</style>").last().unwrap();
    assert_eq!(
        body.matches("class=\"katex-tag\"").count(),
        2,
        "dvě číslované rovnice"
    );

    // chybný label je chyba buildu s řádkem
    fs::write(&input, "text\n\n\\eqref{eq:neni}\n").unwrap();
    let err = format!("{:#}", mdprint::run(&cli(input)).unwrap_err());
    assert!(
        err.contains("neznámý odkaz") && err.contains("řádek 3"),
        "{err}"
    );
}

/// End-to-end s prohlížečem: čísla v obsahu = skutečné stránky nadpisů
/// v PDF pro každou velikost písma. Bez Chrome/Edge se přeskočí.
#[test]
fn toc_pages_match_printed_pdf() {
    let Ok(browser) = mdprint::paginate::find_browser(None) else {
        eprintln!("přeskočeno: Chrome/Edge nenalezen");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let input = doc_with_table_and_figure(dir.path());

    let mut c = cli(input);
    c.toc_pages = true;
    c.lof = true;
    c.lot = true;
    c.pdf = Some(mdprint::cli::PdfMode::All);
    c.browser = Some(browser);
    let out = mdprint::run_full(&c).unwrap();
    assert!(out.warnings.is_empty(), "{:?}", out.warnings);
    assert_eq!(out.pdfs.len(), 3);

    let html = fs::read_to_string(&out.html).unwrap();
    let re = regex::Regex::new(
        r##"<a href="#([^"]+)">.*?<span class="toc-page" data-s="(\d*)" data-m="(\d*)" data-l="(\d*)">"##,
    )
    .unwrap();
    let rows: Vec<_> = re.captures_iter(&html).collect();
    // 6 nadpisů v obsahu + 1 položka seznamu obrázků + 1 položka seznamu tabulek
    assert_eq!(rows.len(), 8, "položky obsahu a seznamů");
    assert_eq!(&rows[6][1], "fig:bod");
    assert_eq!(&rows[7][1], "tbl:data");

    for (i, pdf) in out.pdfs.iter().enumerate() {
        let dests = mdprint::paginate::named_destination_pages(pdf).unwrap();
        for row in &rows {
            let expected: u32 = row[i + 2].parse().expect("číslo strany");
            assert_eq!(
                dests.get(&row[1]),
                Some(&expected),
                "{} v {}",
                &row[1],
                pdf.display()
            );
        }
    }
    // dokument je vícestránkový a velké písmo má víc stran než malé
    let last = |i: usize| rows[4][i + 2].parse::<u32>().unwrap();
    assert!(
        last(0) > 1 && last(2) > last(0),
        "s={} l={}",
        last(0),
        last(2)
    );

    // úklid: jen doc.md, bod.png, doc.html, img_doc a tři PDF
    let names: Vec<String> = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names.len(), 7, "{names:?}");
}

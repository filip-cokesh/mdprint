//! Stránkování přes headless Chrome / Edge: změření čísel stran pro obsah
//! (`--toc-pages`) a export do PDF (`--pdf`).
//!
//! Vlastní odhad stránkování by nebyl věrný (shaping fontů, KaTeX, obrázky,
//! `break-*`, dělení slov), proto mdprint nechá dokument vysázet prohlížečem
//! a z PDF přečte pojmenované cíle (named destinations): Chrome ukládá každou
//! kotvu, na kterou vede odkaz, jako cíl s odkazem na stránku.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow, bail};
use lopdf::{Document, Object};
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, percent_decode, utf8_percent_encode};

use crate::cli::FontSize;
use crate::render::PageTriple;

/// Limit na jeden render (velké dokumenty s KaTeXem trvají jednotky sekund).
const RENDER_TIMEOUT: Duration = Duration::from_secs(180);

/// Najde prohlížeč: zadaná cesta (CLI / MDPRINT_BROWSER / toml) musí
/// existovat; jinak se zkusí obvyklá umístění Chrome, Edge a Chromium.
pub fn find_browser(explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(p) = explicit {
        anyhow::ensure!(p.is_file(), "prohlížeč nenalezen: {}", p.display());
        return Ok(p.to_path_buf());
    }
    candidates()
        .into_iter()
        .find(|p| p.is_file())
        .ok_or_else(|| {
            anyhow!(
                "nenalezen Chrome, Edge ani Chromium — zadejte cestu přes --browser \
                 nebo proměnnou MDPRINT_BROWSER"
            )
        })
}

fn candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if cfg!(windows) {
        let roots = ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"]
            .iter()
            .filter_map(|v| std::env::var_os(v).map(PathBuf::from))
            .collect::<Vec<_>>();
        for rel in [
            r"Google\Chrome\Application\chrome.exe",
            r"Microsoft\Edge\Application\msedge.exe",
            r"Chromium\Application\chrome.exe",
        ] {
            for root in &roots {
                out.push(root.join(rel));
            }
        }
    } else if cfg!(target_os = "macos") {
        for app in [
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
            "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
            "/Applications/Chromium.app/Contents/MacOS/Chromium",
        ] {
            out.push(PathBuf::from(app));
        }
    } else {
        let names = [
            "google-chrome",
            "google-chrome-stable",
            "chromium",
            "chromium-browser",
            "microsoft-edge",
        ];
        if let Some(path) = std::env::var_os("PATH") {
            for dir in std::env::split_paths(&path) {
                for n in names {
                    out.push(dir.join(n));
                }
            }
        }
    }
    out
}

/// Znaky ponechané v cestě `file://` URL (zbytek procentově zakódován).
const PATH_KEEP: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'/')
    .remove(b':')
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

/// Absolutní `file:///…` URL pro prohlížeč (i s diakritikou a mezerami).
pub fn file_url(path: &Path) -> Result<String> {
    let abs = fs::canonicalize(path).with_context(|| format!("nelze najít {}", path.display()))?;
    let mut s = abs.to_string_lossy().replace('\\', "/");
    // Windows canonicalize vrací \\?\D:\… (popř. \\?\UNC\server\…)
    if let Some(rest) = s.strip_prefix("//?/UNC/") {
        s = format!("//{rest}");
    } else if let Some(rest) = s.strip_prefix("//?/") {
        s = rest.to_string();
    }
    let encoded = utf8_percent_encode(&s, PATH_KEEP).to_string();
    Ok(if encoded.starts_with("//") {
        format!("file:{encoded}")
    } else if encoded.starts_with('/') {
        format!("file://{encoded}")
    } else {
        format!("file:///{encoded}")
    })
}

/// Kopie stránky s přepínačem velikosti písma nastaveným na `size`
/// (radio `checked`); print.css podle něj volí velikost i čísla v obsahu.
pub fn with_font_size(page: &str, size: FontSize) -> String {
    let mut out = page.to_string();
    for s in FontSize::ALL {
        let id = format!("id=\"fs-{}\"", s.name());
        out = out.replacen(&format!("{id} checked>"), &format!("{id}>"), 1);
    }
    let id = format!("id=\"fs-{}\"", size.name());
    out.replacen(&format!("{id}>"), &format!("{id} checked>"), 1)
}

/// Dočasné soubory, které se smažou i při chybě.
struct TempFiles(Vec<PathBuf>);

impl Drop for TempFiles {
    fn drop(&mut self) {
        for p in &self.0 {
            let _ = fs::remove_file(p);
        }
    }
}

/// Jeden tiskový úkol: HTML → PDF.
pub struct PrintJob {
    pub html: PathBuf,
    pub pdf: PathBuf,
}

/// Vytiskne všechny úkoly paralelně (každý proces s vlastním profilem, takže
/// nekoliduje s běžícím prohlížečem uživatele).
pub fn print_all(browser: &Path, jobs: &[PrintJob]) -> Result<()> {
    let profiles = tempfile::Builder::new()
        .prefix("mdprint-browser-")
        .tempdir()
        .context("nelze vytvořit dočasnou složku pro prohlížeč")?;
    let mut children: Vec<(Child, &PrintJob)> = Vec::new();
    for (i, job) in jobs.iter().enumerate() {
        let _ = fs::remove_file(&job.pdf);
        let url = file_url(&job.html)?;
        // Chrome relativní cestu k PDF neřeší vůči našemu CWD
        let pdf_abs = std::path::absolute(&job.pdf)
            .with_context(|| format!("neplatná cesta {}", job.pdf.display()))?;
        let child = Command::new(browser)
            .arg("--headless")
            .arg("--disable-gpu")
            .arg("--no-first-run")
            .arg("--no-default-browser-check")
            .arg("--disable-extensions")
            .arg("--no-pdf-header-footer")
            .arg("--print-to-pdf-no-header")
            .arg(format!(
                "--user-data-dir={}",
                profiles.path().join(i.to_string()).display()
            ))
            .arg(format!("--print-to-pdf={}", pdf_abs.display()))
            .arg(&url)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .with_context(|| format!("nelze spustit prohlížeč {}", browser.display()))?;
        children.push((child, job));
    }

    let started = Instant::now();
    let mut failure: Option<anyhow::Error> = None;
    for (mut child, job) in children {
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break Some(status),
                Ok(None) if started.elapsed() > RENDER_TIMEOUT => {
                    let _ = child.kill();
                    let _ = child.wait();
                    break None;
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(50)),
                Err(e) => {
                    failure.get_or_insert(anyhow!("čekání na prohlížeč selhalo: {e}"));
                    break None;
                }
            }
        };
        let ok = status.is_some_and(|s| s.success())
            && fs::metadata(&job.pdf).is_ok_and(|m| m.len() > 0);
        if !ok && failure.is_none() {
            failure = Some(match status {
                None => anyhow!(
                    "tisk {} do PDF nedoběhl do {} s",
                    job.html.display(),
                    RENDER_TIMEOUT.as_secs()
                ),
                Some(s) => anyhow!(
                    "prohlížeč nevytvořil PDF z {} (kód {s})",
                    job.html.display()
                ),
            });
        }
    }
    match failure {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

/// Pojmenované cíle PDF → číslo stránky (od 1). Klíče jsou id kotev
/// (Chrome je ukládá procentově zakódované, např. `%C3%BAvod` = `úvod`).
pub fn named_destination_pages(pdf: &Path) -> Result<HashMap<String, u32>> {
    let doc = Document::load(pdf).with_context(|| format!("nelze číst PDF {}", pdf.display()))?;
    let page_numbers: HashMap<lopdf::ObjectId, u32> =
        doc.get_pages().into_iter().map(|(n, id)| (id, n)).collect();
    let catalog = doc.catalog().context("PDF bez katalogu")?;

    let mut raw: Vec<(Vec<u8>, &Object)> = Vec::new();
    // PDF 1.1: slovník /Dests v katalogu (tak to dělá Chrome/Skia)
    if let Ok(dests) = catalog.get(b"Dests") {
        let (_, dests) = doc.dereference(dests)?;
        for (k, v) in dests.as_dict()?.iter() {
            raw.push((k.clone(), v));
        }
    }
    // PDF 1.2+: strom /Names /Dests
    if let Ok(names) = catalog.get(b"Names") {
        let (_, names) = doc.dereference(names)?;
        if let Ok(tree) = names.as_dict()?.get(b"Dests") {
            collect_name_tree(&doc, tree, &mut raw, 0)?;
        }
    }

    let mut out = HashMap::new();
    for (key, value) in raw {
        let id = percent_decode(&key).decode_utf8_lossy().into_owned();
        if let Some(page) = dest_page(&doc, value)?.and_then(|p| page_numbers.get(&p)) {
            out.insert(id, *page);
        }
    }
    Ok(out)
}

fn collect_name_tree<'a>(
    doc: &'a Document,
    node: &'a Object,
    out: &mut Vec<(Vec<u8>, &'a Object)>,
    depth: usize,
) -> Result<()> {
    if depth > 32 {
        bail!("PDF: příliš hluboký strom jmen");
    }
    let (_, node) = doc.dereference(node)?;
    let dict = node.as_dict()?;
    if let Ok(names) = dict.get(b"Names") {
        let (_, names) = doc.dereference(names)?;
        for pair in names.as_array()?.chunks(2) {
            if let [k, v] = pair {
                out.push((k.as_str()?.to_vec(), v));
            }
        }
    }
    if let Ok(kids) = dict.get(b"Kids") {
        let (_, kids) = doc.dereference(kids)?;
        for kid in kids.as_array()? {
            collect_name_tree(doc, kid, out, depth + 1)?;
        }
    }
    Ok(())
}

/// Cíl = `[stránka /XYZ …]` nebo slovník `<< /D [stránka …] >>`.
fn dest_page(doc: &Document, value: &Object) -> Result<Option<lopdf::ObjectId>> {
    let (_, value) = doc.dereference(value)?;
    let array = match value {
        Object::Array(a) => a,
        Object::Dictionary(d) => match d.get(b"D") {
            Ok(inner) => doc.dereference(inner)?.1.as_array()?,
            Err(_) => return Ok(None),
        },
        _ => return Ok(None),
    };
    Ok(array.first().and_then(|o| o.as_reference().ok()))
}

/// Změří čísla stran všech kotev ve třech velikostech písma.
/// `page` je finální HTML se zástupnými čísly v obsahu; sondy se zapisují
/// vedle výstupu (kvůli relativním cestám k obrázkům) jako skryté soubory.
pub fn measure(
    browser: &Path,
    page: &str,
    out_dir: &Path,
    stem: &str,
) -> Result<HashMap<String, PageTriple>> {
    let pdf_dir = tempfile::Builder::new()
        .prefix("mdprint-probe-")
        .tempdir()
        .context("nelze vytvořit dočasnou složku")?;
    let mut temps = TempFiles(Vec::new());
    let mut jobs = Vec::new();
    for size in FontSize::ALL {
        let html = out_dir.join(format!(".{stem}.mdprint-probe-{}.html", size.name()));
        fs::write(&html, with_font_size(page, size))
            .with_context(|| format!("nelze zapsat {}", html.display()))?;
        temps.0.push(html.clone());
        jobs.push(PrintJob {
            html,
            pdf: pdf_dir.path().join(format!("{}.pdf", size.name())),
        });
    }
    print_all(browser, &jobs)?;

    let mut out: HashMap<String, PageTriple> = HashMap::new();
    for (i, job) in jobs.iter().enumerate() {
        for (id, page) in named_destination_pages(&job.pdf)? {
            out.entry(id).or_default()[i] = Some(page);
        }
    }
    Ok(out)
}

/// Vytiskne hotové HTML do PDF ve zvolených velikostech písma. Varianta
/// shodná s výchozí velikostí HTML se tiskne přímo z výstupního souboru.
pub fn export_pdfs(
    browser: &Path,
    out_html: &Path,
    page: &str,
    default_size: FontSize,
    sizes: &[FontSize],
    pdf_for: impl Fn(FontSize) -> PathBuf,
) -> Result<Vec<PathBuf>> {
    let out_dir = out_html.parent().unwrap_or(Path::new("."));
    let stem = out_html
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut temps = TempFiles(Vec::new());
    let mut jobs = Vec::new();
    for &size in sizes {
        let html = if size == default_size {
            out_html.to_path_buf()
        } else {
            let p = out_dir.join(format!(".{stem}.mdprint-pdf-{}.html", size.name()));
            fs::write(&p, with_font_size(page, size))
                .with_context(|| format!("nelze zapsat {}", p.display()))?;
            temps.0.push(p.clone());
            p
        };
        jobs.push(PrintJob {
            html,
            pdf: pdf_for(size),
        });
    }
    print_all(browser, &jobs)?;
    Ok(jobs.into_iter().map(|j| j.pdf).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn font_size_variant_moves_checked() {
        let page = r#"<input id="fs-small"><input id="fs-normal" checked><input id="fs-large">"#;
        let large = with_font_size(page, FontSize::Large);
        assert_eq!(
            large,
            r#"<input id="fs-small"><input id="fs-normal"><input id="fs-large" checked>"#
        );
        assert_eq!(with_font_size(&large, FontSize::Normal), page);
    }

    #[test]
    fn file_url_encodes_path() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("úvod a #1.html");
        fs::write(&f, "x").unwrap();
        let url = file_url(&f).unwrap();
        assert!(
            url.starts_with("file:///") || url.starts_with("file://"),
            "{url}"
        );
        assert!(url.ends_with("/%C3%BAvod%20a%20%231.html"), "{url}");
        assert!(!url.contains('\\'), "{url}");
    }

    #[test]
    fn explicit_missing_browser_is_error() {
        let err = find_browser(Some(Path::new("/neexistuje/chrome"))).unwrap_err();
        assert!(err.to_string().contains("prohlížeč nenalezen"), "{err}");
    }

    /// Minimální PDF se slovníkem /Dests (jako z Chrome): 2 stránky,
    /// cíl `%C3%BAvod` na 2. stránce, `konec` přes /D slovník na 1.
    #[test]
    fn reads_named_destinations() {
        use lopdf::{Dictionary, Stream, dictionary};
        let mut doc = Document::with_version("1.4");
        let pages_id = doc.new_object_id();
        let mut kids = Vec::new();
        for _ in 0..2 {
            let content = doc.add_object(Stream::new(dictionary! {}, Vec::new()));
            let page = doc.add_object(dictionary! {
                "Type" => "Page",
                "Parent" => pages_id,
                "Contents" => content,
                "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
            });
            kids.push(Object::Reference(page));
        }
        let p1 = kids[0].as_reference().unwrap();
        let p2 = kids[1].as_reference().unwrap();
        doc.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Kids" => kids,
                "Count" => 2,
            }),
        );
        let mut dests = Dictionary::new();
        dests.set(
            "%C3%BAvod",
            vec![
                Object::Reference(p2),
                "XYZ".into(),
                0.into(),
                500.into(),
                0.into(),
            ],
        );
        dests.set(
            "konec",
            dictionary! { "D" => vec![Object::Reference(p1), "Fit".into()] },
        );
        let dests_id = doc.add_object(dests);
        let catalog = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
            "Dests" => dests_id,
        });
        doc.trailer.set("Root", catalog);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.pdf");
        doc.save(&path).unwrap();

        let pages = named_destination_pages(&path).unwrap();
        assert_eq!(pages.get("úvod"), Some(&2));
        assert_eq!(pages.get("konec"), Some(&1));
    }
}

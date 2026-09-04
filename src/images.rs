use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use comrak::nodes::{AstNode, NodeValue};
use deunicode::deunicode;
use percent_encoding::percent_decode_str;
use serde::{Deserialize, Serialize};

const MANIFEST: &str = ".manifest.json";

#[derive(Debug, thiserror::Error)]
pub enum ImageError {
    #[error("obrázek nenalezen: {path} (řádek {line})")]
    Missing { path: String, line: usize },
    #[error("stažení obrázku selhalo: {url} (řádek {line}): {reason}")]
    Fetch {
        url: String,
        line: usize,
        reason: String,
    },
}

#[derive(Serialize, Deserialize, Default, Debug)]
struct Manifest {
    files: BTreeSet<String>,
}

/// Projde AST, zkopíruje lokální obrázky do `img_dir` (jméno `<slug>-<hash8>.<ext>`,
/// dedup podle obsahu) a přepíše URL v AST na relativní cesty. Vzdálené URL nechává.
/// Osiřelé soubory z minulého běhu maže výhradně podle manifestu.
pub fn process<'a>(
    root: &'a AstNode<'a>,
    input_dir: &Path,
    img_dir: &Path,
    img_dir_name: &str,
    fetch: bool,
) -> Result<()> {
    // hash obsahu → cílové jméno souboru (deduplikace)
    let mut by_hash: HashMap<String, String> = HashMap::new();
    // zdrojová cesta → cílové jméno (ušetří opakované hashování téhož odkazu)
    let mut by_source: HashMap<PathBuf, String> = HashMap::new();
    // vzdálená URL → cílové jméno (jedna URL = jedno stažení)
    let mut by_url: HashMap<String, String> = HashMap::new();
    let mut manifest = Manifest::default();

    for node in root.descendants() {
        let mut data = node.data.borrow_mut();
        let line = data.sourcepos.start.line;
        let NodeValue::Image(link) = &mut data.value else {
            continue;
        };
        if is_remote(&link.url) {
            if fetch && !link.url.to_ascii_lowercase().starts_with("data:") {
                let name = fetch_remote(&link.url, line, img_dir, &mut by_url)?;
                manifest.files.insert(name.clone());
                link.url = format!("{img_dir_name}/{name}");
            }
            continue;
        }

        let source = resolve_source(&link.url, input_dir).ok_or(ImageError::Missing {
            path: link.url.clone(),
            line,
        })?;

        let target_name = match by_source.get(&source) {
            Some(name) => name.clone(),
            None => {
                let content = fs::read(&source)
                    .with_context(|| format!("nelze číst obrázek {}", source.display()))?;
                let hash = short_hash(&content);
                let name = match by_hash.get(&hash) {
                    Some(name) => name.clone(),
                    None => {
                        let name = target_name(&source, &hash);
                        fs::create_dir_all(img_dir).with_context(|| {
                            format!("nelze vytvořit složku {}", img_dir.display())
                        })?;
                        fs::write(img_dir.join(&name), &content).with_context(|| {
                            format!("nelze zapsat {}", img_dir.join(&name).display())
                        })?;
                        by_hash.insert(hash, name.clone());
                        name
                    }
                };
                by_source.insert(source, name.clone());
                name
            }
        };

        manifest.files.insert(target_name.clone());
        link.url = format!("{img_dir_name}/{target_name}");
    }

    sweep_orphans(img_dir, &manifest)?;

    if !manifest.files.is_empty() {
        let json = serde_json::to_string_pretty(&manifest)?;
        fs::write(img_dir.join(MANIFEST), json)
            .with_context(|| format!("nelze zapsat manifest v {}", img_dir.display()))?;
    }
    Ok(())
}

/// Stáhne vzdálený obrázek do složky obrázků. Cílové jméno je deterministické
/// z URL (`<slug>-<hash8 z URL>.<ext>`), takže existující soubor slouží jako
/// disková cache — při dalším buildu se nestahuje znovu.
fn fetch_remote(
    url: &str,
    line: usize,
    img_dir: &Path,
    by_url: &mut HashMap<String, String>,
) -> Result<String> {
    if let Some(name) = by_url.get(url) {
        return Ok(name.clone());
    }
    let name = remote_target_name(url);
    let target = img_dir.join(&name);
    if !target.is_file() {
        let bytes = download(url).map_err(|reason| ImageError::Fetch {
            url: url.to_string(),
            line,
            reason,
        })?;
        fs::create_dir_all(img_dir)
            .with_context(|| format!("nelze vytvořit složku {}", img_dir.display()))?;
        fs::write(&target, bytes).with_context(|| format!("nelze zapsat {}", target.display()))?;
    }
    by_url.insert(url.to_string(), name.clone());
    Ok(name)
}

/// Strop velikosti stahovaného obrázku (ochrana paměti a disku).
const FETCH_LIMIT: u64 = 50 * 1024 * 1024;

fn download(url: &str) -> std::result::Result<Vec<u8>, String> {
    let mut response = ureq::get(url).call().map_err(|e| e.to_string())?;
    response
        .body_mut()
        .with_config()
        .limit(FETCH_LIMIT)
        .read_to_vec()
        .map_err(|e| format!("{e} (limit {} MB)", FETCH_LIMIT / 1024 / 1024))
}

/// Jméno pro stažený soubor: slug z posledního segmentu cesty URL + 8 znaků
/// hashe celé URL (deterministické bez stažení) + přípona z URL, je-li rozumná.
/// Veřejné kvůli integračním testům diskové cache.
pub fn remote_target_name(url: &str) -> String {
    let hash = short_hash(url.as_bytes());
    let path = url
        .split(['?', '#'])
        .next()
        .unwrap_or(url)
        .trim_end_matches('/');
    let last = path.rsplit('/').next().unwrap_or("");
    let (stem, ext) = match last.rsplit_once('.') {
        Some((s, e))
            if !e.is_empty() && e.len() <= 5 && e.chars().all(|c| c.is_ascii_alphanumeric()) =>
        {
            (s, Some(e.to_ascii_lowercase()))
        }
        _ => (last, None),
    };
    let slug = slugify(stem);
    match ext {
        Some(ext) => format!("{slug}-{hash}.{ext}"),
        None => format!("{slug}-{hash}"),
    }
}

fn is_remote(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("data:")
}

/// Rozloží MD cestu na existující soubor: dekóduje `%20` apod., srovná zpětná
/// lomítka, relativní cesty řeší vůči adresáři vstupu (ne CWD). Vrací None,
/// pokud soubor neexistuje.
fn resolve_source(url: &str, input_dir: &Path) -> Option<PathBuf> {
    let decoded = percent_decode_str(url).decode_utf8().ok()?;
    let cleaned = decoded.replace('\\', "/");
    let path = Path::new(cleaned.as_str());
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        input_dir.join(path)
    };
    // canonicalize ověří existenci a vstřebá `..`
    let canonical = absolute.canonicalize().ok()?;
    canonical.is_file().then_some(canonical)
}

fn short_hash(content: &[u8]) -> String {
    blake3::hash(content).to_hex()[..8].to_string()
}

fn target_name(source: &Path, hash: &str) -> String {
    let stem = source
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let ext = source
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase());
    let slug = slugify(&stem);
    match ext {
        Some(ext) if !ext.is_empty() => format!("{slug}-{hash}.{ext}"),
        _ => format!("{slug}-{hash}"),
    }
}

/// Slug: bez diakritiky, malými písmeny, bez mezer; ne-alfanumerické runy → `-`.
fn slugify(stem: &str) -> String {
    let ascii = deunicode(stem).to_ascii_lowercase();
    let mut out = String::with_capacity(ascii.len());
    let mut pending_dash = false;
    for c in ascii.chars() {
        if c.is_ascii_alphanumeric() {
            if pending_dash && !out.is_empty() {
                out.push('-');
            }
            pending_dash = false;
            out.push(c);
        } else {
            pending_dash = true;
        }
    }
    if out.is_empty() { "img".into() } else { out }
}

/// Smaže ze složky pouze soubory uvedené v předchozím manifestu, které v novém
/// buildu nefigurují. Nikdy nemaže celou složku ani cizí soubory.
fn sweep_orphans(img_dir: &Path, current: &Manifest) -> Result<()> {
    let manifest_path = img_dir.join(MANIFEST);
    let Ok(old_json) = fs::read_to_string(&manifest_path) else {
        return Ok(());
    };
    let old: Manifest = serde_json::from_str(&old_json)
        .with_context(|| format!("poškozený manifest {}", manifest_path.display()))?;
    for name in old.files.difference(&current.files) {
        // jen prosté jméno souboru — žádné cesty ven ze složky
        if Path::new(name)
            .file_name()
            .map(|f| f == Path::new(name).as_os_str())
            != Some(true)
        {
            continue;
        }
        let victim = img_dir.join(name);
        if victim.is_file() {
            fs::remove_file(&victim)
                .with_context(|| format!("nelze smazat osiřelý {}", victim.display()))?;
        }
    }
    if current.files.is_empty() {
        // žádné obrázky v novém buildu → starý manifest už neplatí
        let _ = fs::remove_file(&manifest_path);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_removes_diacritics_spaces_and_case() {
        assert_eq!(slugify("Průřez Nosníku č. 12"), "prurez-nosniku-c-12");
        assert_eq!(slugify("obrázek"), "obrazek");
        assert_eq!(slugify("a__b--c"), "a-b-c");
        assert_eq!(slugify("---"), "img");
        assert_eq!(slugify(""), "img");
    }

    #[test]
    fn target_name_keeps_lowercase_extension() {
        let name = target_name(Path::new("C:/x/Foto Zdi.PNG"), "abcd1234");
        assert_eq!(name, "foto-zdi-abcd1234.png");
    }

    #[test]
    fn remote_names_are_deterministic_and_clean() {
        let a = remote_target_name("https://ex.cz/cesta/Graf%20A.png?v=2#f");
        let b = remote_target_name("https://ex.cz/cesta/Graf%20A.png?v=2#f");
        assert_eq!(a, b);
        assert!(a.starts_with("graf-20a-") && a.ends_with(".png"), "{a}");
        // jiná URL (i jen query) = jiné jméno
        let c = remote_target_name("https://ex.cz/cesta/Graf%20A.png?v=3");
        assert_ne!(a, c);
        // bez rozumné přípony jméno bez přípony
        let d = remote_target_name("https://ex.cz/api/obrazek");
        assert!(d.starts_with("obrazek-") && !d.contains('.'), "{d}");
    }

    #[test]
    fn resolve_handles_percent_backslash_and_dotdot() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("sub");
        fs::create_dir(&sub).unwrap();
        let img = dir.path().join("můj obrázek.png");
        fs::write(&img, b"png").unwrap();

        let from_sub = resolve_source("../m%C5%AFj%20obr%C3%A1zek.png", &sub).unwrap();
        assert_eq!(from_sub, img.canonicalize().unwrap());

        let backslash = resolve_source("..\\můj obrázek.png", &sub).unwrap();
        assert_eq!(backslash, img.canonicalize().unwrap());

        assert!(resolve_source("neexistuje.png", dir.path()).is_none());
    }
}

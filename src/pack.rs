//! Template packy — externí šablony načítané za běhu z disku.
//!
//! Pack je složka: `pack.toml` (metadata, [company] defaulty, [[fonts]],
//! [[links]] pro odkazy v patičce), `template.css` (vrstva vkládaná za
//! print.css), volitelně `logo-light.png` a `logo-dark.png`. Jde o vědomou
//! výjimku z pravidla „žádné čtení z disku za běhu" — vestavěná default
//! šablona zůstává plně embedovaná.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::config::Company;

pub const MANIFEST_FILE: &str = "pack.toml";
pub const CSS_FILE: &str = "template.css";
pub const LOGO_LIGHT_FILE: &str = "logo-light.png";
pub const LOGO_DARK_FILE: &str = "logo-dark.png";

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct Manifest {
    pack: Meta,
    #[serde(default)]
    company: Company,
    #[serde(default)]
    fonts: Vec<FontDecl>,
    #[serde(default)]
    links: Vec<LinkDecl>,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct Meta {
    name: String,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct FontDecl {
    file: String,
    family: String,
    weight: u16,
    #[serde(default)]
    style: Option<String>,
}

/// Odkaz v patičce (`[[links]]` v pack.toml); ikona je volitelný SVG/PNG
/// soubor uvnitř složky packu.
#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct LinkDecl {
    label: String,
    url: String,
    #[serde(default)]
    icon: Option<String>,
}

#[derive(Debug)]
pub struct PackFont {
    pub family: String,
    pub weight: u16,
    pub style: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug)]
pub struct PackLink {
    pub label: String,
    pub url: String,
    /// Ikona jako base64 data URI (`image/svg+xml` nebo `image/png`).
    pub icon_data_uri: Option<String>,
}

#[derive(Debug)]
pub struct TemplatePack {
    pub name: String,
    pub css: String,
    pub fonts: Vec<PackFont>,
    pub links: Vec<PackLink>,
    pub logo_light: Option<Vec<u8>>,
    pub logo_dark: Option<Vec<u8>>,
    /// Defaulty údajů firmy z pack.toml; `[company]` v mdprint.toml je přebíjí.
    pub company: Company,
}

impl TemplatePack {
    pub fn load(dir: &Path) -> Result<Self> {
        anyhow::ensure!(dir.is_dir(), "složka šablony nenalezena: {}", dir.display());
        let manifest_path = dir.join(MANIFEST_FILE);
        let manifest_text = fs::read_to_string(&manifest_path)
            .with_context(|| format!("chybí manifest šablony {}", manifest_path.display()))?;
        let manifest: Manifest = toml::from_str(&manifest_text)
            .with_context(|| format!("chyba v {}", manifest_path.display()))?;

        let css_path = dir.join(CSS_FILE);
        let css = fs::read_to_string(&css_path)
            .with_context(|| format!("chybí CSS šablony {}", css_path.display()))?;

        let pack_root = dir
            .canonicalize()
            .with_context(|| format!("nelze rozložit cestu packu {}", dir.display()))?;
        let mut fonts = Vec::new();
        for decl in manifest.fonts {
            let bytes = read_pack_file(dir, &pack_root, &decl.file, "font šablony")?;
            fonts.push(PackFont {
                family: decl.family,
                weight: decl.weight,
                style: decl.style.unwrap_or_else(|| "normal".into()),
                bytes,
            });
        }

        let mut links = Vec::new();
        for decl in manifest.links {
            let icon_data_uri = match &decl.icon {
                Some(rel) => {
                    let mime = match Path::new(rel).extension().and_then(|e| e.to_str()) {
                        Some(e) if e.eq_ignore_ascii_case("svg") => "image/svg+xml",
                        Some(e) if e.eq_ignore_ascii_case("png") => "image/png",
                        _ => anyhow::bail!(
                            "ikona odkazu {rel} v {} musí být .svg nebo .png",
                            manifest_path.display()
                        ),
                    };
                    let bytes = read_pack_file(dir, &pack_root, rel, "ikona odkazu")?;
                    Some(crate::assets::image_data_uri(mime, &bytes))
                }
                None => None,
            };
            links.push(PackLink {
                label: decl.label,
                url: decl.url,
                icon_data_uri,
            });
        }

        Ok(TemplatePack {
            name: manifest.pack.name,
            css,
            fonts,
            links,
            logo_light: read_optional(&dir.join(LOGO_LIGHT_FILE))?,
            logo_dark: read_optional(&dir.join(LOGO_DARK_FILE))?,
            company: manifest.company,
        })
    }

    /// `@font-face` bloky fontů packu (base64 data URI, jako u embedovaných).
    pub fn fonts_css(&self) -> String {
        let mut css = String::new();
        for f in &self.fonts {
            css.push_str(&crate::assets::font_face_css(
                &f.family, f.weight, &f.style, &f.bytes,
            ));
        }
        css
    }
}

/// Přečte soubor deklarovaný v pack.toml; soubory packu musí ležet uvnitř
/// jeho složky (žádné `../` ven). `what` je český popisek do chybové hlášky.
fn read_pack_file(dir: &Path, pack_root: &Path, rel: &str, what: &str) -> Result<Vec<u8>> {
    let path = dir.join(rel);
    let canonical = path
        .canonicalize()
        .with_context(|| format!("chybí {what} {}", path.display()))?;
    anyhow::ensure!(
        canonical.starts_with(pack_root),
        "{what} {rel} leží mimo složku packu — cesty v pack.toml musí zůstat uvnitř"
    );
    fs::read(&canonical).with_context(|| format!("chybí {what} {}", path.display()))
}

fn read_optional(path: &Path) -> Result<Option<Vec<u8>>> {
    if !path.is_file() {
        return Ok(None);
    }
    fs::read(path)
        .map(Some)
        .with_context(|| format!("nelze číst {}", path.display()))
}

/// Sloučení údajů firmy: hodnoty z mdprint.toml přebíjejí defaulty packu.
pub fn merge_company(pack: &Company, toml: &Company) -> Company {
    Company {
        name: toml.name.clone().or_else(|| pack.name.clone()),
        address: toml.address.clone().or_else(|| pack.address.clone()),
        ico: toml.ico.clone().or_else(|| pack.ico.clone()),
        dic: toml.dic.clone().or_else(|| pack.dic.clone()),
        web: toml.web.clone().or_else(|| pack.web.clone()),
        email: toml.email.clone().or_else(|| pack.email.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demo_pack_dir() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pack-demo")
    }

    #[test]
    fn loads_demo_pack() {
        let pack = TemplatePack::load(&demo_pack_dir()).unwrap();
        assert_eq!(pack.name, "demo");
        assert_eq!(pack.company.name.as_deref(), Some("Demo s.r.o."));
        assert!(pack.css.contains(".brand-header"));
        assert!(pack.logo_light.is_some() && pack.logo_dark.is_some());
        assert_eq!(pack.fonts.len(), 1);
        assert!(pack.fonts_css().contains("base64,d09G"));
        assert_eq!(pack.links.len(), 2);
        assert_eq!(pack.links[0].label, "GitHub");
        assert!(
            pack.links[0]
                .icon_data_uri
                .as_deref()
                .unwrap()
                .starts_with("data:image/svg+xml;base64,")
        );
        assert!(pack.links[1].icon_data_uri.is_none());
    }

    #[test]
    fn link_icon_cannot_escape_pack_dir_and_rejects_odd_extension() {
        let dir = tempfile::tempdir().unwrap();
        let pack = dir.path().join("pack");
        std::fs::create_dir(&pack).unwrap();
        std::fs::write(dir.path().join("tajny.svg"), b"<svg/>").unwrap();
        std::fs::write(pack.join("template.css"), "/* */").unwrap();

        std::fs::write(
            pack.join("pack.toml"),
            "[pack]\nname = \"zly\"\n\n[[links]]\nlabel = \"X\"\nurl = \"https://x.invalid\"\nicon = \"../tajny.svg\"\n",
        )
        .unwrap();
        let err = TemplatePack::load(&pack).unwrap_err();
        assert!(err.to_string().contains("mimo složku packu"), "{err}");

        std::fs::write(pack.join("ikona.gif"), b"GIF89a").unwrap();
        std::fs::write(
            pack.join("pack.toml"),
            "[pack]\nname = \"zly\"\n\n[[links]]\nlabel = \"X\"\nurl = \"https://x.invalid\"\nicon = \"ikona.gif\"\n",
        )
        .unwrap();
        let err = TemplatePack::load(&pack).unwrap_err();
        assert!(err.to_string().contains("musí být .svg nebo .png"), "{err}");
    }

    #[test]
    fn missing_dir_and_manifest_give_readable_errors() {
        let err = TemplatePack::load(Path::new("neexistuje-slozka")).unwrap_err();
        assert!(
            err.to_string().contains("složka šablony nenalezena"),
            "{err}"
        );

        let dir = tempfile::tempdir().unwrap();
        let err = TemplatePack::load(dir.path()).unwrap_err();
        assert!(
            format!("{err:#}").contains("chybí manifest šablony"),
            "{err:#}"
        );
    }

    #[test]
    fn font_path_cannot_escape_pack_dir() {
        let dir = tempfile::tempdir().unwrap();
        let pack = dir.path().join("pack");
        std::fs::create_dir(&pack).unwrap();
        std::fs::write(dir.path().join("tajny.woff2"), b"data").unwrap();
        std::fs::write(
            pack.join("pack.toml"),
            "[pack]\nname = \"zly\"\n\n[[fonts]]\nfile = \"../tajny.woff2\"\nfamily = \"X\"\nweight = 400\n",
        )
        .unwrap();
        std::fs::write(pack.join("template.css"), "/* */").unwrap();

        let err = TemplatePack::load(&pack).unwrap_err();
        assert!(err.to_string().contains("mimo složku packu"), "{err}");
    }

    #[test]
    fn company_merge_prefers_toml() {
        let pack = Company {
            name: Some("Pack s.r.o.".into()),
            web: Some("pack.example".into()),
            ..Company::default()
        };
        let toml = Company {
            name: Some("Toml a.s.".into()),
            ico: Some("123".into()),
            ..Company::default()
        };
        let merged = merge_company(&pack, &toml);
        assert_eq!(merged.name.as_deref(), Some("Toml a.s."));
        assert_eq!(merged.web.as_deref(), Some("pack.example"));
        assert_eq!(merged.ico.as_deref(), Some("123"));
    }
}

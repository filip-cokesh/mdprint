use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::cli::{Cli, Lang};
use crate::parse::FrontMatter;

/// `mdprint.toml` vedle vstupu (nebo `--config`). Všechna pole volitelná.
#[derive(Deserialize, Default, Debug)]
#[serde(deny_unknown_fields)]
pub struct FileConfig {
    pub lang: Option<Lang>,
    /// "default", nebo cesta ke složce template packu (relativně k tomuto tomlu)
    pub template: Option<String>,
    pub paper: Option<Paper>,
    pub figures: Option<Figures>,
    pub fonts: Option<Fonts>,
    pub company: Option<Company>,
}

/// Údaje firmy pro patičku brandované šablony (`[company]`).
#[derive(Deserialize, Default, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Company {
    pub name: Option<String>,
    pub address: Option<String>,
    pub ico: Option<String>,
    pub dic: Option<String>,
    pub web: Option<String>,
    pub email: Option<String>,
}

#[derive(Deserialize, Default, Debug)]
#[serde(deny_unknown_fields)]
pub struct Paper {
    /// Např. "A4" (výchozí)
    pub size: Option<String>,
    /// Např. "2.5cm 2cm" (výchozí)
    pub margin: Option<String>,
}

#[derive(Deserialize, Default, Debug)]
#[serde(deny_unknown_fields)]
pub struct Figures {
    /// Prefix popisku, výchozí "Obr."
    pub prefix: Option<String>,
    /// Číslovat obrázky (výchozí true)
    pub numbering: Option<bool>,
}

#[derive(Deserialize, Default, Debug)]
#[serde(deny_unknown_fields)]
pub struct Fonts {
    pub serif: Option<String>,
    pub sans: Option<String>,
    pub mono: Option<String>,
}

/// Výsledná konfigurace po sloučení: CLI > front matter > mdprint.toml > výchozí.
#[derive(Debug)]
pub struct Config {
    pub lang: Lang,
    pub paper_size: String,
    pub paper_margin: String,
    pub figure_prefix: String,
    pub figure_numbering: bool,
    pub font_serif: String,
    pub font_sans: String,
    pub font_mono: String,
    /// Složka template packu; `None` = vestavěná default šablona.
    pub pack_dir: Option<PathBuf>,
    /// `[company]` z mdprint.toml (přebíjí defaulty z pack.toml).
    pub company: Company,
    pub toc: bool,
    pub fetch: bool,
}

impl Config {
    pub fn resolve(
        cli: &Cli,
        front_matter: Option<&FrontMatter>,
        input_dir: &Path,
    ) -> Result<Self> {
        let (file_cfg, config_dir) = load_file_config(cli, input_dir)?;

        let lang = cli
            .lang
            .or_else(|| front_matter.and_then(|fm| fm.lang))
            .or(file_cfg.lang)
            .unwrap_or(Lang::Cs);

        // šablona: relativní cesta se řeší vůči místu, kde byla zapsaná
        let template_choice: Option<(String, PathBuf)> = cli
            .template
            .clone()
            .map(|t| {
                let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                (t, cwd)
            })
            .or_else(|| {
                front_matter
                    .and_then(|fm| fm.template.clone())
                    .map(|t| (t, input_dir.to_path_buf()))
            })
            .or_else(|| file_cfg.template.map(|t| (t, config_dir)));
        let pack_dir = match template_choice {
            None => None,
            Some((t, _)) if t == "default" => None,
            Some((t, base)) => {
                let p = Path::new(&t);
                Some(if p.is_absolute() {
                    p.to_path_buf()
                } else {
                    base.join(p)
                })
            }
        };

        let company = file_cfg.company.unwrap_or_default();

        let paper = file_cfg.paper.unwrap_or_default();
        let figures = file_cfg.figures.unwrap_or_default();
        let fonts = file_cfg.fonts.unwrap_or_default();

        Ok(Config {
            lang,
            paper_size: paper.size.unwrap_or_else(|| "A4".into()),
            paper_margin: paper.margin.unwrap_or_else(|| "2.5cm 2cm".into()),
            // default popisku figur podle jazyka; [figures] prefix má přednost
            figure_prefix: figures.prefix.unwrap_or_else(|| {
                match lang {
                    Lang::Cs => "Obr.",
                    Lang::En => "Fig.",
                    Lang::De => "Abb.",
                }
                .into()
            }),
            figure_numbering: figures.numbering.unwrap_or(true),
            font_serif: fonts.serif.unwrap_or_else(|| "Libertinus Serif".into()),
            font_sans: fonts.sans.unwrap_or_else(|| "Libertinus Sans".into()),
            font_mono: fonts.mono.unwrap_or_else(|| "JetBrains Mono".into()),
            pack_dir,
            company,
            toc: cli.toc,
            fetch: cli.fetch,
        })
    }
}

/// Vrací konfiguraci + složku, ze které pochází (pro relativní cesty v ní).
fn load_file_config(cli: &Cli, input_dir: &Path) -> Result<(FileConfig, PathBuf)> {
    let path = match &cli.config {
        Some(p) => {
            anyhow::ensure!(p.is_file(), "konfigurace nenalezena: {}", p.display());
            p.clone()
        }
        None => {
            let default = input_dir.join("mdprint.toml");
            if !default.is_file() {
                return Ok((FileConfig::default(), input_dir.to_path_buf()));
            }
            default
        }
    };
    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("nelze číst konfiguraci {}", path.display()))?;
    let cfg =
        toml::from_str(&text).with_context(|| format!("chyba v konfiguraci {}", path.display()))?;
    Ok((cfg, dir))
}

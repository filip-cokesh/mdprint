use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::cli::{Cli, FontSize, Lang, PdfMode};
use crate::parse::FrontMatter;
use crate::xref::EqNumbering;

/// `mdprint.toml` vedle vstupu (nebo `--config`). Všechna pole volitelná.
#[derive(Deserialize, Default, Debug)]
#[serde(deny_unknown_fields)]
pub struct FileConfig {
    pub lang: Option<Lang>,
    /// "default", nebo cesta ke složce template packu (relativně k tomuto tomlu)
    pub template: Option<String>,
    pub paper: Option<Paper>,
    pub figures: Option<Figures>,
    pub tables: Option<Tables>,
    pub equations: Option<Equations>,
    pub fonts: Option<Fonts>,
    pub math: Option<Math>,
    pub print: Option<Print>,
    pub company: Option<Company>,
}

/// Tisk (`[print]`): číslování stránek, čísla v obsahu, výchozí velikost písma.
#[derive(Deserialize, Default, Debug)]
#[serde(deny_unknown_fields)]
pub struct Print {
    /// Číslo stránky „3 / 8“ v patičce tisku (výchozí true)
    pub page_numbers: Option<bool>,
    /// Čísla stran v obsahu (výchozí false; vyžaduje Chrome/Edge)
    pub toc_pages: Option<bool>,
    /// small | normal | large (výchozí normal)
    pub font_size: Option<FontSize>,
    /// Cesta k prohlížeči (relativně k tomuto tomlu)
    pub browser: Option<PathBuf>,
}

/// Velikost matematiky (`[math]`), v em vůči okolnímu textu.
#[derive(Deserialize, Default, Debug)]
#[serde(deny_unknown_fields)]
pub struct Math {
    /// Inline vzorce (výchozí 0.95 — kalibrováno na Libertinus)
    pub inline: Option<f64>,
    /// Blokové vzorce (výchozí 1.0)
    pub display: Option<f64>,
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
    /// Seznam obrázků na konci dokumentu (výchozí false)
    pub list: Option<bool>,
}

/// Tabulky s popiskem (`[tables]`): odstavec „Tabulka: …“ před tabulkou.
#[derive(Deserialize, Default, Debug)]
#[serde(deny_unknown_fields)]
pub struct Tables {
    /// Prefix popisku, výchozí Tab. / Table / Tab. podle jazyka
    pub prefix: Option<String>,
    /// Seznam tabulek na konci dokumentu (výchozí false)
    pub list: Option<bool>,
}

/// Číslování rovnic (`[equations]`).
#[derive(Deserialize, Default, Debug)]
#[serde(deny_unknown_fields)]
pub struct Equations {
    /// labeled (výchozí) | all | none
    pub numbering: Option<EqNumbering>,
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
    /// Prefix popisku tabulky („Tab.“ / „Table“ / „Tab.“).
    pub table_prefix: String,
    /// Které blokové rovnice číslovat.
    pub eq_numbering: EqNumbering,
    /// Seznam obrázků / tabulek na konci dokumentu.
    pub lof: bool,
    pub lot: bool,
    pub font_serif: String,
    pub font_sans: String,
    pub font_mono: String,
    /// Velikost inline matematiky v em (výchozí 0.95).
    pub math_inline: f64,
    /// Velikost blokové matematiky v em (výchozí 1.0).
    pub math_display: f64,
    /// Složka template packu; `None` = vestavěná default šablona.
    pub pack_dir: Option<PathBuf>,
    /// `[company]` z mdprint.toml (přebíjí defaulty z pack.toml).
    pub company: Company,
    pub toc: bool,
    /// Čísla stran v obsahu (implikuje `toc`).
    pub toc_pages: bool,
    /// Číslo stránky v patičce tisku.
    pub page_numbers: bool,
    /// Výchozí velikost písma (radio v HTML, velikost jednoho PDF).
    pub font_size: FontSize,
    /// Výstup do PDF (`None` = jen HTML).
    pub pdf: Option<PdfMode>,
    /// Prohlížeč zadaný uživatelem (CLI > MDPRINT_BROWSER > toml).
    pub browser: Option<PathBuf>,
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
            .or_else(|| file_cfg.template.map(|t| (t, config_dir.clone())));
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

        let print = file_cfg.print.unwrap_or_default();
        let fm = |f: fn(&FrontMatter) -> Option<bool>| front_matter.and_then(f);
        let toc_pages = cli.toc_pages || fm(|f| f.toc_pages).or(print.toc_pages).unwrap_or(false);
        let page_numbers = if cli.no_page_numbers {
            false
        } else {
            fm(|f| f.page_numbers)
                .or(print.page_numbers)
                .unwrap_or(true)
        };
        let font_size = cli
            .font_size
            .or_else(|| front_matter.and_then(|f| f.font_size))
            .or(print.font_size)
            .unwrap_or_default();
        let browser = cli
            .browser
            .clone()
            .or_else(|| std::env::var_os("MDPRINT_BROWSER").map(PathBuf::from))
            .or_else(|| {
                print.browser.map(|b| {
                    if b.is_absolute() {
                        b
                    } else {
                        config_dir.join(b)
                    }
                })
            });

        let paper = file_cfg.paper.unwrap_or_default();
        let figures = file_cfg.figures.unwrap_or_default();
        let tables = file_cfg.tables.unwrap_or_default();
        let equations = file_cfg.equations.unwrap_or_default();
        let fonts = file_cfg.fonts.unwrap_or_default();
        let lof = cli.lof || fm(|f| f.lof).or(figures.list).unwrap_or(false);
        let lot = cli.lot || fm(|f| f.lot).or(tables.list).unwrap_or(false);

        let math = file_cfg.math.unwrap_or_default();
        let math_inline = math.inline.unwrap_or(0.95);
        let math_display = math.display.unwrap_or(1.0);
        for (name, value) in [("inline", math_inline), ("display", math_display)] {
            anyhow::ensure!(
                (0.5..=2.0).contains(&value),
                "[math] {name} = {value} je mimo rozsah 0.5–2.0 (velikost v em)"
            );
        }

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
            table_prefix: tables
                .prefix
                .unwrap_or_else(|| crate::xref::default_table_prefix(lang).into()),
            eq_numbering: equations.numbering.unwrap_or_default(),
            lof,
            lot,
            font_serif: fonts.serif.unwrap_or_else(|| "Libertinus Serif".into()),
            font_sans: fonts.sans.unwrap_or_else(|| "Libertinus Sans".into()),
            font_mono: fonts.mono.unwrap_or_else(|| "JetBrains Mono".into()),
            math_inline,
            math_display,
            pack_dir,
            company,
            toc: cli.toc || toc_pages,
            toc_pages,
            page_numbers,
            font_size,
            pdf: cli.pdf,
            browser,
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

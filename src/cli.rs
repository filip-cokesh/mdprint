use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use serde::Deserialize;

/// Převodník Markdown → tiskově kvalitní HTML (jedna binárka, vše inlinované kromě obrázků).
#[derive(Parser, Debug, Default)]
#[command(name = "mdprint", version, about)]
pub struct Cli {
    /// Vstupní Markdown soubor
    pub input: PathBuf,

    /// Výstupní HTML soubor (výchozí: vedle vstupu, stejný název s .html)
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Jazyk dokumentu (přebíjí front matter i mdprint.toml)
    #[arg(long, value_enum)]
    pub lang: Option<Lang>,

    /// Vygenerovat obsah (TOC) za hlavičku dokumentu
    #[arg(long)]
    pub toc: bool,

    /// Doplnit do obsahu čísla stran pro tisk (zapíná --toc; vyžaduje
    /// Chrome nebo Edge, stránkování změří ve třech velikostech písma)
    #[arg(long)]
    pub toc_pages: bool,

    /// Seznam obrázků na konci dokumentu (s --toc-pages i čísla stran)
    #[arg(long)]
    pub lof: bool,

    /// Seznam tabulek na konci dokumentu (jen tabulky s popiskem
    /// „Tabulka: …“ / „Table: …“ / „Tabelle: …“ v odstavci před tabulkou)
    #[arg(long)]
    pub lot: bool,

    /// Výchozí velikost písma (přepínač A/A/A v HTML i velikost v --pdf)
    #[arg(long, value_enum)]
    pub font_size: Option<FontSize>,

    /// Nečíslovat stránky v tisku
    #[arg(long)]
    pub no_page_numbers: bool,

    /// Uložit i PDF vedle HTML (vyžaduje Chrome nebo Edge);
    /// `--pdf=all` uloží tři PDF, pro každou velikost písma jedno
    #[arg(long, value_enum, num_args = 0..=1, require_equals = true, default_missing_value = "one")]
    pub pdf: Option<PdfMode>,

    /// Cesta k prohlížeči pro --pdf a --toc-pages (jinak MDPRINT_BROWSER
    /// nebo automatické hledání Chrome / Edge / Chromium)
    #[arg(long)]
    pub browser: Option<PathBuf>,

    /// Stáhnout vzdálené obrázky (http/https) do složky obrázků
    #[arg(long)]
    pub fetch: bool,

    /// Cesta ke konfiguraci (výchozí: mdprint.toml vedle vstupu)
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// Šablona vzhledu: "default", nebo cesta ke složce template packu
    /// (přebíjí front matter i mdprint.toml)
    #[arg(long)]
    pub template: Option<String>,
}

#[derive(ValueEnum, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    Cs,
    En,
    De,
}

/// Velikost písma — odpovídá přepínači A/A/A (radio `fs-small|normal|large`).
#[derive(ValueEnum, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
#[serde(rename_all = "lowercase")]
pub enum FontSize {
    Small,
    #[default]
    Normal,
    Large,
}

impl FontSize {
    pub const ALL: [FontSize; 3] = [FontSize::Small, FontSize::Normal, FontSize::Large];

    /// Název pro id radia, šablonu a jména souborů (`small|normal|large`).
    pub fn name(self) -> &'static str {
        match self {
            FontSize::Small => "small",
            FontSize::Normal => "normal",
            FontSize::Large => "large",
        }
    }
}

/// `--pdf` (jedno PDF ve zvolené velikosti) nebo `--pdf=all` (tři PDF).
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum PdfMode {
    One,
    All,
}

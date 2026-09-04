use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use serde::Deserialize;

/// Převodník Markdown → tiskově kvalitní HTML (jedna binárka, vše inlinované kromě obrázků).
#[derive(Parser, Debug)]
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

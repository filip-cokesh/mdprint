pub mod assets;
pub mod cli;
pub mod config;
pub mod highlight;
pub mod hyphen;
pub mod images;
pub mod math;
pub mod pack;
pub mod paginate;
pub mod parse;
pub mod render;
pub mod typo;

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use comrak::Arena;

use cli::Cli;
use config::Config;

/// Výsledek běhu: zapsané HTML, případná PDF a varování (nefatální).
#[derive(Debug, Default)]
pub struct RunOutput {
    pub html: PathBuf,
    pub pdfs: Vec<PathBuf>,
    pub warnings: Vec<String>,
}

/// Celý běh: MD → AST → obrázky → HTML. Vrací cestu k zapsanému HTML.
pub fn run(cli: &Cli) -> Result<PathBuf> {
    run_full(cli).map(|o| o.html)
}

/// Celý běh včetně čísel stran v obsahu (`--toc-pages`) a PDF (`--pdf`).
pub fn run_full(cli: &Cli) -> Result<RunOutput> {
    anyhow::ensure!(
        cli.input.is_file(),
        "vstupní soubor nenalezen: {}",
        cli.input.display()
    );
    let input_dir = cli
        .input
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    let source = fs::read_to_string(&cli.input)
        .with_context(|| format!("nelze číst {}", cli.input.display()))?;

    let opts = parse::options();
    let arena = Arena::new();
    let root = parse::parse(&arena, &source, &opts);

    let front_matter =
        parse::front_matter(root).with_context(|| format!("{}", cli.input.display()))?;
    let cfg = Config::resolve(cli, front_matter.as_ref(), &input_dir)?;

    let out_html = cli
        .output
        .clone()
        .unwrap_or_else(|| cli.input.with_extension("html"));
    let out_dir = out_html
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let stem = out_html
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .context("výstupní cesta nemá jméno souboru")?;
    let img_dir_name = format!("img_{stem}");

    typo::apply(root, cfg.lang)?;
    hyphen::Hyphenator::new(cfg.lang)?.apply(root);
    math::MathRenderer::with_sizes(cfg.math_inline, cfg.math_display).render_all(root)?;
    highlight::Highlighter::new().highlight_all(root)?;

    images::process(
        root,
        &input_dir,
        &out_dir.join(&img_dir_name),
        &img_dir_name,
        cfg.fetch,
    )?;

    let template_pack = cfg
        .pack_dir
        .as_deref()
        .map(pack::TemplatePack::load)
        .transpose()?;

    let title = parse::document_title(front_matter.as_ref(), root, &stem);
    let toc_entries = if cfg.toc {
        render::toc_entries(root)
    } else {
        Vec::new()
    };
    let toc_mode = if cfg.toc_pages {
        render::TocPages::Placeholder
    } else {
        render::TocPages::Off
    };
    let toc = render::render_toc(&toc_entries, cfg.lang, toc_mode);
    let body = render::body_html(root, &opts, &cfg)?;
    let page = render::page_html(
        &title,
        front_matter.as_ref(),
        &cfg,
        template_pack.as_ref(),
        toc.as_deref(),
        &body,
    )?;

    fs::create_dir_all(&out_dir)
        .with_context(|| format!("nelze vytvořit {}", out_dir.display()))?;

    let mut warnings = Vec::new();
    let mut page = page;
    // čísla stran v obsahu: tři sondy se zástupnými čísly → doplnit čísla;
    // při selhání obsah bez čísel (HTML je i tak použitelné) + varování
    if let (true, Some(placeholder_toc)) = (cfg.toc_pages, toc.as_deref()) {
        let measured = paginate::find_browser(cfg.browser.as_deref())
            .and_then(|b| paginate::measure(&b, &page, &out_dir, &stem));
        let final_toc = match &measured {
            Ok(map) => render::render_toc(&toc_entries, cfg.lang, render::TocPages::Numbers(map)),
            Err(e) => {
                warnings.push(format!("čísla stran v obsahu vynechána: {e:#}"));
                render::render_toc(&toc_entries, cfg.lang, render::TocPages::Off)
            }
        }
        .expect("obsah má položky");
        page = page.replacen(placeholder_toc, &final_toc, 1);
    }

    fs::write(&out_html, &page).with_context(|| format!("nelze zapsat {}", out_html.display()))?;

    let mut pdfs = Vec::new();
    if let Some(mode) = cfg.pdf {
        let browser = paginate::find_browser(cfg.browser.as_deref())
            .context("--pdf vyžaduje Chrome nebo Edge")?;
        let sizes: Vec<cli::FontSize> = match mode {
            cli::PdfMode::One => vec![cfg.font_size],
            cli::PdfMode::All => cli::FontSize::ALL.to_vec(),
        };
        let pdf_for = |size: cli::FontSize| match mode {
            cli::PdfMode::One => out_html.with_extension("pdf"),
            cli::PdfMode::All => out_dir.join(format!("{stem}-{}.pdf", size.name())),
        };
        pdfs = paginate::export_pdfs(&browser, &out_html, &page, cfg.font_size, &sizes, pdf_for)?;
    }

    Ok(RunOutput {
        html: out_html,
        pdfs,
        warnings,
    })
}

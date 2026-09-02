pub mod assets;
pub mod cli;
pub mod config;
pub mod highlight;
pub mod hyphen;
pub mod images;
pub mod math;
pub mod pack;
pub mod parse;
pub mod render;
pub mod typo;

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use comrak::Arena;

use cli::Cli;
use config::Config;

/// Celý běh: MD → AST → obrázky → HTML. Vrací cestu k zapsanému HTML.
pub fn run(cli: &Cli) -> Result<PathBuf> {
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

    typo::apply(root, cfg.lang);
    hyphen::Hyphenator::new(cfg.lang)?.apply(root);
    math::MathRenderer::new().render_all(root)?;
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
    let toc = if cfg.toc {
        render::build_toc(root, cfg.lang)
    } else {
        None
    };
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
    fs::write(&out_html, page).with_context(|| format!("nelze zapsat {}", out_html.display()))?;
    Ok(out_html)
}

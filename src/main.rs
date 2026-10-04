use clap::Parser;
use mdprint::cli::Cli;

fn main() {
    let cli = Cli::parse();
    match mdprint::run_full(&cli) {
        Ok(out) => {
            for w in &out.warnings {
                eprintln!("mdprint: varování: {w}");
            }
            println!("mdprint: zapsáno {}", out.html.display());
            for pdf in &out.pdfs {
                println!("mdprint: zapsáno {}", pdf.display());
            }
        }
        Err(err) => {
            eprintln!("mdprint: chyba: {err:#}");
            std::process::exit(1);
        }
    }
}

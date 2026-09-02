use clap::Parser;
use mdprint::cli::Cli;

fn main() {
    let cli = Cli::parse();
    match mdprint::run(&cli) {
        Ok(out) => println!("mdprint: zapsáno {}", out.display()),
        Err(err) => {
            eprintln!("mdprint: chyba: {err:#}");
            std::process::exit(1);
        }
    }
}

fn main() {
    // Ikona a verzní metadata exe jen pro Windows target; build script běží
    // na hostu, winresource je dependency jen při cílení na Windows.
    #[cfg(windows)]
    {
        let icon = "assets/logo/mdprint.ico";
        if std::env::var_os("CARGO_CFG_WINDOWS").is_some() && std::path::Path::new(icon).is_file() {
            let mut res = winresource::WindowsResource::new();
            res.set_icon(icon);
            res.set("ProductName", "mdprint");
            res.set(
                "FileDescription",
                "Markdown to print-quality self-contained HTML",
            );
            res.set("LegalCopyright", "Copyright (c) 2026 Filip Hokeš");
            if let Err(e) = res.compile() {
                println!("cargo:warning=ikona exe se nevložila: {e}");
            }
        }
        println!("cargo:rerun-if-changed=assets/logo/mdprint.ico");
    }
}

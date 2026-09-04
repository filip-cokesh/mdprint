# mdprint

CLI převodník Markdown → tiskově kvalitní HTML (jedna statická binárka, vše
inlinované kromě obrázků). Primární jazyk dokumentů je čeština, česká
mikrotypografie je klíčová funkce.

## Příkazy

```
cargo test                          # unit + integrační testy (insta snapshoty)
cargo clippy --all-targets -- -D warnings
cargo fmt
cargo run -- <vstup.md>             # dev běh
cargo build --release --target x86_64-pc-windows-msvc
```

Limit velikosti release binárky: **20 MB**.

## Architektura (datový tok)

`main` → `config` (mdprint.toml + CLI merge; priority CLI > front matter > toml)
→ `parse` (comrak AST; extensions: table, footnotes, description_lists,
math_dollars, front_matter `---`, header_id_prefix; `render.unsafe` zapnuté kvůli
HTML z math/highlight) → transformace AST v pořadí **typo → hyphen → math →
highlight → images** → `render` (vlastní formatter → HTML → minijinja šablona).

- `cli.rs` — clap derive; `--template` = "default" nebo cesta k packu
- `config.rs` — `FileConfig` (mdprint.toml, deny_unknown_fields) → `Config`;
  řešení cesty packu relativně k místu deklarace (CLI→CWD, toml→složka tomlu,
  front matter→složka vstupu)
- `parse.rs` — comrak options, YAML front matter (title/author/date/lang/
  version/template; fallback titulku na první H1)
- `typo/` — typografie cs/en/de výhradně nad `NodeValue::Text` (fancy-regex):
  `nbsp.rs` (vč. DIN zkratek a de dat), `quotes.rs` (`double_low_quotes` sdílené
  cs+de, anglické zvlášť), `dash.rs` (spaced en dash cs/de, em dash en),
  `misc.rs` (apostrof, ×, …, tisíce); tři pipeline v `typo::transform`
- jazykové defaulty generovaných textů: figure prefix Obr./Fig./Abb.
  (config.rs), TOC Obsah/Contents/Inhalt, verze/version/Version, datum byline
  ČSN/beze změny/DIN (render.rs)
- `hyphen.rs` — U+00AD přes hyphenation (embed_all); slovo ≥ 6 znaků, ≥ 3 před/za,
  nadpisy se nedělí
- `math.rs` — katex-rs: `NodeValue::Math` → `HtmlInline`; chybný TeX = chyba
  s řádkem. Pozn.: katex-rs má nedeterministické pořadí atributů — snapshot testy
  normalizují (viz `tests/snapshots.rs::normalize`)
- `highlight.rs` — syntect (default-fancy), per-line `<span class="line">`
  s vyvažováním spanů přes hranice řádků (`wrap_lines`)
- `images.rs` — kopie do `img_<název>/` jako `<slug>-<blake3/8hex>.<ext>`, dedup
  podle obsahu, `.manifest.json` (úklid maže jen osiřelé z manifestu); `--fetch`
  přes ureq, jméno deterministicky z URL = disková cache
- `pack.rs` — **template packy**: externí šablony načítané za běhu (vědomá
  výjimka z pravidla „vše embedované"); složka s pack.toml + template.css
  + volitelnými logy a fonty; `[company]` defaulty z packu přebíjí mdprint.toml
- `render.rs` — `create_formatter!` (figure/figcaption „Obr. N"), minijinja
  šablona (brand hlavička/patička `.brand-*` při aktivním packu), TOC přes
  comrak `Anchorizer`, české datum s U+00A0
- `assets.rs` — `include_str!`/`include_bytes!` všeho vestavěného; fonty
  Libertinus + JetBrains Mono + KaTeX jako base64 `@font-face`

Tmavý režim bez JS: tokeny `--c-*` v screen.css, `prefers-color-scheme`
+ CSS-only přepínač `#theme-invert` (`:has()`); tisk vždy světlý. Pozor na
specificitu při skrývání variant log (`.brand-header .brand-logo-dark`).

## Konvence

- Kód a komentáře: identifikátory anglicky, doc-komentáře česky; uživatelské
  chybové hlášky česky, vždy s cestou, u obsahu s číslem řádku (sourcepos)
- Zakázáno: `unsafe`, JS runtime i JS ve výstupu, C knihovny nad rámec
  vynucených závislostí
- Chyby: `anyhow` s `.context()`, typované `thiserror` tam, kde se testují
- Testy: tabulkové unit testy u modulů, integrační v `tests/pipeline.rs`,
  insta snapshoty v `tests/snapshots.rs` (dummy pack: `tests/fixtures/pack-demo/`)
- Do fixture a testů nepatří žádná reálná firemní identita

## Backlog

- Konfigurovatelná velikost matematiky v mdprint.toml (nyní screen.css:
  inline `.katex` 0.95em, display 1em)
- Tmavé téma pro bloky kódu (nyní záměrně světlé „ostrůvky" v obou režimech —
  vyžadovalo by druhé syntect téma se scopovanými selektory)

<img src="assets/logo/mdprint-logo.svg" width="96" align="left" alt="mdprint logo">

# mdprint

**Markdown → print-quality, self-contained HTML. One static binary.**

<br clear="left">

[![crates.io](https://img.shields.io/crates/v/mdprint.svg)](https://crates.io/crates/mdprint)
[![CI](https://github.com/filip-cokesh/mdprint/actions/workflows/ci.yml/badge.svg)](https://github.com/filip-cokesh/mdprint/actions/workflows/ci.yml)
[![license](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

*Česká verze dokumentace: [README.cs.md](README.cs.md)*

## Why

Pandoc is a cannon for this job — a universal converter with a LaTeX toolchain
behind it when you want beautiful PDF. Printing straight from the browser is
the opposite extreme: zero setup, but no typesetting — no widow control, no
non-breaking spaces, tables split mid-row. And no existing tool cares about
Czech microtypography at all. mdprint sits in the middle: one binary turns
a Markdown file into a single HTML file that reads well on the web *and*
prints beautifully from Chrome (Ctrl+P → PDF or paper), with Czech typographic
rules applied by default.

## Install

```
cargo install mdprint
```

or grab a prebuilt binary from [Releases](https://github.com/filip-cokesh/mdprint/releases).

## Quick start

```
mdprint dokument.md
```

This produces `dokument.html` — a single file with all CSS, fonts and
build-time-rendered KaTeX inlined — plus an `img_dokument/` folder with the
referenced images (deduplicated by content hash). Open the HTML anywhere,
print it from Chrome, or publish it as-is.

## Features

- **Single static binary** — no runtime dependencies, no JavaScript in the
  output, fonts (Libertinus, JetBrains Mono) embedded as WOFF2
- **KaTeX math** rendered at build time (`$…$`, `$$…$$`) via a native Rust
  port — invalid TeX fails the build with a line number
- **Syntax highlighting** — static `<span>`s (syntect), light theme suited
  for print
- **Czech typography** applied on the AST — see below
- **Hyphenation** with embedded Czech and English patterns (soft hyphens,
  words ≥ 6 chars, ≥ 3 chars around each break, headings exempt)
- **Print CSS** — A4 page setup, orphans/widows control, table headers
  repeated across pages, unbreakable figures and table rows, long code lines
  wrapped with hanging indent, URLs printed after external links
- **Image handling** — paths resolved relative to the input, copies named
  `<slug>-<hash>`, cleanup driven by a manifest (mdprint never deletes
  anything it didn't create), optional `--fetch` for remote images with
  a disk cache
- **Dark mode without JavaScript** — follows `prefers-color-scheme`, manual
  CSS-only toggle, print always stays light
- **Template packs** — external branding (logo header, colors, fonts,
  company footer) loaded from a folder at run time, no recompilation

## Czech typography

The distinguishing feature. Rules follow ČSN 01 6910 and Pecina's *Knihy
a typografie*, are applied only to text nodes (never code, math or URLs),
and each rule is a separately tested function.

| Rule | Before | After |
|---|---|---|
| Non-breaking space after single-letter prepositions | `šel k lesu` | `šel k lesu` (`k` U+00A0 `lesu`) |
| Number–unit binding | `25 °C`, `10 kg`, `100 %` | bound with U+00A0 |
| Abbreviations | `č. 12`, `např. zde` | bound with U+00A0 |
| Initials | `J. K. Tyl` | bound with U+00A0 |
| Dates | `1. 1. 2026`, `5. července` | bound with U+00A0 |
| Czech quotes | `"slovo"`, `'vnořené'` | `„slovo“`, `‚vnořené‘` |
| Dash with spaces | `text - vsuvka` | `text – vsuvka` (NBSP before dash) |
| Number ranges | `10-20`, `1918-1938` | `10–20` (en dash) |
| Ellipsis | `atd...` | `atd…` |
| Multiplication | `2 x 3`, `40x60` | `2 × 3`, `40×60` (hex `0x1F` untouched) |
| Thousands groups | `1 000 000` | narrow NBSP U+202F |

With `--lang en` the Czech rules are disabled and English quotes and
hyphenation patterns are used instead.

## Configuration

Optional `mdprint.toml` next to the input file (or `--config <path>`).
CLI flags override it; every key is optional.

```toml
lang = "cs"                     # cs | en; front matter `lang:` overrides this
template = "../my-pack"         # "default" or path to a template pack folder

[paper]
size = "A4"                     # anything valid for CSS @page size
margin = "2.5cm 2cm"

[figures]
prefix = "Obr."                 # figure caption prefix: "Obr. 1: …"
numbering = true

[fonts]                         # primary family overrides (embedded stay as fallback)
serif = "Libertinus Serif"
sans = "Libertinus Sans"
mono = "JetBrains Mono"

[company]                       # footer of a branded template pack;
name = "…"                      # overrides the pack's own defaults
address = "…"
ico = "…"                       # company registration number
dic = "…"                       # VAT number
web = "…"
email = "…"
```

Documents may carry YAML front matter (`title`, `author`, `date`, `version`,
`lang`, `template`) — `title` renders a document header, ISO dates are
typeset the Czech way (`2. 9. 2026`), `version` shows in the byline.

A **template pack** is a folder with `pack.toml`, `template.css` and optional
logos (`logo-light.png`, `logo-dark.png`) and fonts; pass it via
`--template <folder>`. The output stays a single HTML file — pack assets are
inlined like the built-in ones. See the pack section in
[README.cs.md](README.cs.md) for the manifest schema.

## CLI reference

```
Usage: mdprint [OPTIONS] <INPUT>

Arguments:
  <INPUT>  Input Markdown file

Options:
  -o, --output <OUTPUT>      Output HTML file (default: next to input, .html)
      --lang <LANG>          Document language [possible values: cs, en]
      --toc                  Generate a table of contents (headings 1-3)
      --fetch                Download remote images (with a disk cache)
      --config <CONFIG>      Config path (default: mdprint.toml next to input)
      --template <TEMPLATE>  "default" or path to a template pack folder
  -h, --help                 Print help
  -V, --version              Print version
```

Exit code is non-zero on any error; missing images and invalid TeX report
the source line.

## Build from source

```
cargo build --release
cargo test
```

Pure Rust, no C dependencies — `syntect` runs with the `default-fancy`
feature (fancy-regex instead of oniguruma), so cross-platform builds need
nothing beyond a Rust toolchain (MSRV 1.88).

## License

Licensed under either of [MIT](LICENSE-MIT) or
[Apache-2.0](LICENSE-APACHE), at your option. Bundled fonts and KaTeX assets
keep their own licenses — see [THIRD-PARTY-LICENSES](THIRD-PARTY-LICENSES).

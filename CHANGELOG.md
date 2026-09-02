# Changelog

All notable changes to this project will be documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.2.1] - 2026-09-02

First release published to crates.io.

### Added
- Project logo; the Windows executable now carries an icon and version
  metadata (`build.rs` + `winresource`)
- English `README.md` (Czech version moved to `README.cs.md`),
  `CHANGELOG.md`, dual licensing (MIT OR Apache-2.0)
- CI (fmt, clippy, tests on Linux/Windows/macOS) and a release workflow
  building binaries for four targets with SHA256 checksums
- crates.io metadata (keywords, categories, MSRV 1.88, package excludes)

## [0.2.0] - 2026-09-02

### Added
- **Template packs**: external branding templates loaded at run time from
  a folder (`--template <dir>`: `pack.toml`, `template.css`, optional logos
  and fonts, `[company]` defaults merged with `mdprint.toml`)
- `THIRD-PARTY-LICENSES` with full font and KaTeX license texts

### Changed
- The built-in corporate template was removed from the core in favour of
  template packs (breaking: the `--template` enum is now a path)

## [0.1.0] - 2026-09-02

Initial version.

### Added
- Markdown → single self-contained HTML (CommonMark + GFM via comrak);
  images copied next to the output with content-hash names and
  a manifest-driven cleanup
- KaTeX math rendered at build time (native Rust port, no JS runtime)
- Syntax highlighting with per-line spans (syntect, `default-fancy`)
- Czech microtypography applied on the AST (non-breaking spaces, quotes,
  dashes, ranges, ellipsis, multiplication sign, thousands groups)
- Hyphenation with embedded Czech and English patterns
- Print CSS (A4, orphans/widows, repeating table headers, hanging indent
  for wrapped code), screen CSS with an academic look
- Embedded Libertinus and JetBrains Mono fonts (WOFF2, base64)
- YAML front matter (title, author, date, version, lang), `mdprint.toml`
  configuration, `--toc`, `--fetch` with a disk cache
- Dark mode without JavaScript (`prefers-color-scheme` + CSS-only toggle)

# Changelog

All notable changes to this project will be documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
- `[toc] numbering = false` turns off the "2.1" counters in the TOC, for
  documents whose headings carry their own numbers (#17).
- `[toc] depth` (1–6, default 3) sets the deepest heading level in the
  TOC (#17).
- `--toc-pages` warns about TOC / list entries whose anchor is missing in
  the measured PDF instead of silently leaving them without a number (#17).

### Fixed
- Print: code blocks were double-spaced — the newline between per-line
  spans added an empty line under `pre-wrap` (#16).
- Print: the line-continuation backslash in bash blocks was set on its own
  line — syntect gives it the class `line`, which matched the print rule
  for mdprint's line spans (#15).

## [0.9.0] - 2026-10-22

### Added
- **Equation numbering** (LaTeX syntax): `\label{eq:x}` anywhere in a
  display formula gives it a number, typeset by KaTeX as `\tag` on the
  right. By default only labelled equations are numbered
  (`[equations] numbering = "labeled"`); `"all"` numbers every `$$…$$`
  with `\notag` / `\nonumber` as opt-out. An equation with its own `\tag`
  keeps it.
- **Cross-references** in text: `\ref{x}` → bare number, `\eqref{eq:x}`
  → "(3)", `\autoref{x}` → localized prefix + number ("rovnice (3)",
  "Obr. 2", "Tab. 1" / "Eq. (3)", "Fig. 2", "Table 1" / "Gl. (3)",
  "Abb. 2", "Tab. 1"), all as links. Figures take a label via
  `![…](img.png){#fig:x}`; unknown or duplicate labels fail the build
  with the source line.
- **Table captions**: a paragraph starting with `Table:` / `Tabulka:` /
  `Tabelle:` immediately before a table becomes its caption above the
  table ("Tab. 1: …"), optionally labelled with a trailing `{#tbl:x}`.
  Prefix configurable via `[tables] prefix`.
- **List of figures / list of tables** (`--lof`, `--lot`; front matter
  `lof:` / `lot:`; `[figures] list`, `[tables] list`) at the end of the
  document, on a new page in print. With `--toc-pages` they get measured
  page numbers from the same headless render as the TOC.
- Figures, captioned tables and numbered equations carry `id` anchors
  (label, or `fig-N` / `tab-N` / `eq-N`).

## [0.8.1] - 2026-10-04

### Fixed
- **Headless browser could hang on macOS** (`--pdf`, `--toc-pages`): a
  fresh browser profile touched the Keychain and never finished. The
  browser now starts with a mock keychain and without background
  services; a complete PDF that stops growing is accepted even if the
  process does not exit, and failures quote the browser's own output.
  0.8.0 was not published to crates.io because of this

## [0.8.0] - 2026-10-04

### Added
- **Page numbers in print**: "3 / 8" bottom right on every page (CSS
  `@page` margin box, Chrome/Edge 131+). On by default; turn off with
  `--no-page-numbers` or `page_numbers = false`
- **Page references in the table of contents** (`--toc-pages`): the
  document is laid out in headless Chrome/Edge in all three font sizes,
  the page of every heading is read from the PDF's named destinations
  and written into the TOC with dot leaders. Print shows the set that
  matches the selected font size. Without a browser mdprint warns and
  keeps the TOC without numbers
- **PDF export**: `--pdf` writes `<name>.pdf` next to the HTML in the
  configured font size, `--pdf=all` writes one PDF per font size
- **Initial font size**: `--font-size`, `font_size` in front matter or
  `[print]` sets which A/A/A option starts selected (and the `--pdf` size)
- `[print]` section in `mdprint.toml` (`page_numbers`, `toc_pages`,
  `font_size`, `browser`); browser path also via `--browser` or
  `MDPRINT_BROWSER`

## [0.7.1] - 2026-10-02

### Fixed
- **Broken math layout**: fractions, sub/superscripts, limits and `\vec`
  arrows overlapped or were misplaced, because the embedded KaTeX 0.18 CSS
  uses prefixed class names (`.katex-sizing`, `.katex-strut`, …) that the
  katex-rs 0.2 renderer did not emit. A regression test now checks that
  every layout-critical class emitted by the renderer has a CSS rule

### Changed
- Math renderer upgraded to katex-rs 0.3.0, which matches KaTeX 0.18
  (embedded CSS and fonts stay at KaTeX 0.18.5)

## [0.7.0] - 2026-09-12

### Added
- **Dark theme for code blocks**: in dark mode, fenced code is now
  highlighted with Base16 Ocean Dark on a matching dark panel instead of
  staying a light island. Print always keeps the light InspiredGitHub
  theme, whatever the on-screen mode

## [0.6.0] - 2026-09-12

### Added
- **Configurable math size**: a `[math]` section in `mdprint.toml` with
  `inline` and `display` (em relative to the surrounding text, range
  0.5–2.0; defaults 0.95/1.0 stay calibrated for Libertinus). The
  page-fit shrink of over-wide formulas in print scales along, so
  enlarged math still fits the page

## [0.5.0] - 2026-09-12

### Added
- **Font-size switcher** (A/A/A next to the theme toggle, CSS-only, no
  JavaScript): small (−12.5 %) / default / large (+15 %) type. The choice
  applies to screen and print alike (large prints at 12 pt, small at
  9.5 pt), the selected A is shown bold, and the controls themselves never
  appear in print. Line wrapping and hyphenation reflow naturally — soft
  hyphens are part of the text, so no re-rendering is needed

### Changed
- Tooltips of the theme and font-size switchers are localized (cs/en/de);
  the theme tooltip was previously hard-coded Czech

## [0.4.0] - 2026-09-08

### Added
- **Footer links in template packs** (`[[links]]` in `pack.toml`): label,
  URL and an optional SVG/PNG icon from the pack folder, rendered as a row
  of links in the branded footer. Icons are inlined as data URIs; monochrome
  icons are inverted to white in dark mode. Icon paths get the same
  inside-the-pack guard as fonts
- A forkable starter pack:
  [mdprint-pack-starter](https://github.com/filip-cokesh/mdprint-pack-starter)

## [0.3.3] - 2026-09-07

### Fixed
- Tall glyphs in display math (integral signs, sum limits) could get
  clipped on screen at certain zoom levels: the horizontal-scroll container
  (`overflow-y: hidden`) cut off glyph ink extending past its 0.1em
  padding. The scroll container now carries generous inner vertical
  padding (with the outer margin reduced to keep the rhythm), so glyph
  ink always fits inside the clip edge

## [0.3.2] - 2026-09-07

### Fixed
- Display formulas wider than the printed page are no longer clipped:
  print allows line breaks at top-level operators (where KaTeX permits
  them) and over-wide formulas get stepped shrink classes (`math-wide-1/2/3`)
  computed from a build-time width estimate; overly long inline formulas
  shrink slightly. The leftover scrollbar in print preview is gone
  (visible+hidden overflow combination computed to `auto`)

## [0.3.1] - 2026-09-04

Security-hardening release (no functional changes for well-formed input).

### Fixed
- Template pack font paths can no longer escape the pack directory
  (`file = "../…"` is now a build error)
- A pathological input paragraph exceeding the regex backtracking limit
  now produces a readable error with the line number instead of a panic
- `--fetch` downloads are explicitly capped at 50 MB

### Added
- `SECURITY.md`, a Security section in both READMEs, full BSD license
  texts for comrak and deunicode in `THIRD-PARTY-LICENSES`, explicit
  minimal CI permissions

## [0.3.0] - 2026-09-04

### Added
- **German language support** (`--lang de`): DIN 5008 typography — „Gänsefüßchen“
  quotes, protected spaces inside multi-part abbreviations (z. B., d. h.,
  i. d. R.), references bound to numbers (Nr. 5, S. 12), German dates
  (numeric and month names), Gedankenstrich, narrow no-break thousands
  groups, German hyphenation patterns, DIN date in the byline (DD.MM.YYYY),
  localized generated texts (Inhalt, Abb., Version)
- **Full English typography**: em dashes (` - `, ` -- `, `--` → `—`,
  Chicago style), number ranges with en dash, multiplication sign,
  number–unit binding; localized defaults (Contents, Fig.)
- Typographic apostrophe (`don't`, `geht's`, `d'Artagnan` → `’`) in all
  languages
- Language-aware default figure prefix (Obr. / Fig. / Abb.) — explicit
  `[figures] prefix` still wins
- English and German showcase documents (live demos)

### Changed
- Czech mode now also converts intra-word apostrophes to `’`

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

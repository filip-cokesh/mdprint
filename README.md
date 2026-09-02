# mdprint

*Markdown → print-quality, self-contained HTML. One static binary, Czech
microtypography built in, KaTeX rendered at build time, no JavaScript in the
output. Docs below are in Czech (the tool's primary audience).*

Převodník Markdown → tiskově kvalitní HTML. Jedna statická binárka bez runtime
závislostí; výstupem je jeden HTML soubor se vším inlinovaným (CSS, fonty,
KaTeX) kromě obrázků, které se kopírují do složky vedle něj. Výstup slouží
rovnocenně webu i tisku z Chromu (Ctrl+P → PDF či papír). Primární jazyk je
čeština — česká mikrotypografie je klíčová funkce.

## Použití

```
mdprint <vstup.md> [-o <výstup.html>] [--lang cs|en] [--toc] [--fetch]
        [--template default|<složka packu>] [--config <soubor>]
```

- výchozí výstup: vedle vstupu, stejný název s `.html`; obrázky do `img_<název>\`
- `--toc` — vygeneruje obsah (nadpisy 1–3) za hlavičku dokumentu
- `--fetch` — stáhne vzdálené obrázky (`http(s)://`) do složky obrázků
  s diskovou cache; bez něj zůstávají URL beze změny
- `--lang` přebíjí `lang:` ve front matteru i `mdprint.toml`; výchozí `cs`
- `--template <složka>` — externí **template pack** (viz níže): brandovaná
  hlavička s logem a názvem firmy, vlastní barvy/fonty, patička. `default`
  = vestavěná šablona. Lze zvolit i ve front matteru (`template: "…"`)
  nebo v toml (priorita CLI > front matter > toml; relativní cesta se řeší
  vůči místu, kde je zapsaná)
- návratový kód ≠ 0 při jakékoli chybě; chybějící obrázek nebo vadný TeX
  je chyba buildu s číslem řádku

## Front matter (volitelný)

```markdown
---
title: Název dokumentu
author: Jméno Autora
date: 2026-09-02
lang: cs
version: "1.2"
---
```

Vše volitelné. `title` vytvoří hlavičku dokumentu (bez něj se titulek bere
z prvního H1, jen do `<title>`); ISO datum se česky sází jako `2. 9. 2026`;
`version` se vypíše v podtitulku jako „verze 1.2“. Verzi pište v uvozovkách —
holé `1.20` by YAML přečetl jako číslo 1.2 (holá čísla ale fungují taky).

## Konfigurace (`mdprint.toml` vedle vstupu, nebo `--config`)

```toml
lang = "cs"

[paper]
size = "A4"            # cokoli platného pro CSS @page size
margin = "2.5cm 2cm"

[figures]
prefix = "Obr."        # popisek figur: „Obr. 1: …“
numbering = true

[fonts]                # přepis primárních rodin (embedované zůstávají fallback)
serif = "Libertinus Serif"
sans = "Libertinus Sans"
mono = "JetBrains Mono"

template = "../moje-sablona"   # volitelně: trvalé zapnutí template packu

[company]              # patička brandované šablony; přebíjí defaulty z packu
name = "…"
address = "…"
ico = "…"
dic = "…"
web = "…"
email = "…"
```

CLI přepínače konfiguraci přebíjejí.

## Template packy

Vlastní vzhled bez rekompilace: pack je složka, kterou předáš přes
`--template`. Výstup zůstává jeden HTML soubor — CSS, fonty i loga packu se
inlinují stejně jako vestavěné assety.

```
muj-pack/
├─ pack.toml           # povinný manifest
├─ template.css        # povinné: CSS vrstva vkládaná ZA vestavěné styly
├─ logo-light.png      # volitelné: logo pro světlý režim a tisk
├─ logo-dark.png       # volitelné: logo pro tmavý režim
└─ fonts/*.woff2       # volitelné: fonty deklarované v pack.toml
```

```toml
# pack.toml
[pack]
name = "muj-pack"

[company]              # defaulty pro hlavičku/patičku (mdprint.toml je přebíjí)
name = "Moje firma s.r.o."

[[fonts]]
file = "fonts/MujFont-Bold.woff2"
family = "Můj Font"
weight = 700
style = "normal"       # volitelné, výchozí "normal"
```

Aktivní pack vloží před dokument `<header class="brand-header">` (loga +
`.brand-name`) a za něj `<footer class="brand-footer">` s údaji `[company]`
— vzhled obojího řídí `template.css` packu (třídy `.brand-*`). Šablona může
přepisovat CSS tokeny vestavěných stylů (`--c-link`, `--font-sans`, …)
a reagovat na tmavý režim stejnými selektory jako vestavěné CSS
(`prefers-color-scheme` + `#theme-invert`).

## Co výstup umí

- CommonMark + GFM: tabulky, footnotes, description listy, škrtnutí
- matematika `$…$` a `$$…$$` — KaTeX vyrenderovaný při buildu (katex-rs,
  žádný JavaScript za běhu)
- zvýraznění kódu — statické `<span>` (syntect), světlé téma vhodné pro tisk
- `![Popisek](…)` samostatně v odstavci → `<figure>` s číslovaným popiskem
- obrázky: cesty relativně ke vstupu, kopie deduplikované podle obsahu jako
  `<slug>-<hash>.<ext>`, úklid řízený `.manifest.json` (nikdy se nemaže nic,
  co mdprint nevytvořil)
- česká typografie nad AST: nezlomitelné mezery (předložky, číslo–jednotka,
  zkratky, iniciály, data, §), uvozovky „…“ ‚…‘, pomlčky a rozsahy, `…`, `×`,
  tisícové skupiny úzkou mezerou; kódu, matematiky a URL se nedotýká
- dělení slov měkkými rozdělovníky (vzory cs/en embedované; slovo ≥ 6 znaků,
  ≥ 3 znaky před i za zlomem, nadpisy se nedělí)
- tmavý režim bez JavaScriptu: automaticky dle systému (`prefers-color-scheme`),
  ruční přepínač ◐ vpravo nahoře (CSS-only checkbox); tisk je vždy světlý,
  bloky kódu zůstávají světlé i v tmavém režimu
- tiskové CSS: A4 s okraji, orphans/widows, nedělené figury a řádky tabulek,
  opakování hlavičky tabulky, zalamování dlouhých řádků kódu s visutou
  indentací, URL za externími odkazy

## Build

```
cargo build --release --target x86_64-pc-windows-msvc
cargo test
cargo clippy --all-targets -- -D warnings
```

Fonty: Libertinus 7.051, JetBrains Mono 2.304, KaTeX 0.18.5 (vše OFL/MIT,
WOFF2 embedované v binárce). Licence třetích stran: `THIRD-PARTY-LICENSES`.

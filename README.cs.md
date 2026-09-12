<img src="assets/logo/mdprint-logo.svg" width="96" align="left" alt="logo mdprint">

# mdprint

**Markdown → tiskově kvalitní, samostatné HTML. Jedna statická binárka.**

<br clear="left">

*English documentation: [README.md](README.md)*

**Živá dema** — skutečné dokumenty z `mdprint --toc` s matematikou,
tabulkami, kódem, obrázky, typografií a tmavým režimem:
[čeština](https://filip-cokesh.github.io/mdprint/) ·
[English](https://filip-cokesh.github.io/mdprint/en.html) ·
[Deutsch](https://filip-cokesh.github.io/mdprint/de.html).
Zkuste i tisk (Ctrl+P).

## Proč

Pandoc je na tuhle úlohu dělo — univerzální konvertor s LaTeXovou sazbou
v zádech. Tisk přímo z prohlížeče je opačný extrém: žádná instalace, ale
žádná sazba — sirotci a vdovy, tabulky zlomené vejpůl, rovné uvozovky.
A českou mikrotypografii neřeší vůbec nikdo. mdprint stojí uprostřed:
jedna binárka udělá z Markdownu jeden HTML soubor, který se dobře čte na
webu *a* krásně tiskne z Chromu (Ctrl+P → PDF či papír), s českými
typografickými pravidly jako výchozím chováním.

## Instalace

```
cargo install mdprint
```

nebo hotová binárka z [Releases](https://github.com/filip-cokesh/mdprint/releases).

## Rychlý start

```
mdprint dokument.md
```

Vznikne `dokument.html` — jeden soubor se vším inlinovaným (CSS, fonty,
KaTeX vyrenderovaný při buildu) — a složka `img_dokument\` se zkopírovanými
obrázky (deduplikace podle hashe obsahu). HTML otevřeš kdekoli, vytiskneš
z Chromu, nebo rovnou zveřejníš.

## Co umí

- **jedna statická binárka** — žádné runtime závislosti, žádný JavaScript ve
  výstupu, fonty (Libertinus, JetBrains Mono) embedované jako WOFF2
- **KaTeX matematika** při buildu (`$…$`, `$$…$$`) — nativní Rust port;
  chybný TeX = chyba buildu s číslem řádku. KaTeX neumí display math lámat,
  proto mdprint v tisku povolí zlom na top-level operátorech a příliš
  široké vzorce stupňovitě zmenší, aby se nic neuřízlo; plnou kontrolu
  dají prostředí `aligned`/`split` s ručními `\\`
- **zvýraznění kódu** — statické `<span>` (syntect); světlé téma na
  obrazovce i v tisku, v tmavém režimu tmavé (Base16 Ocean Dark)
- **typografie pro češtinu, němčinu i angličtinu** nad AST (viz tabulky
  v [README.md](README.md#typography)): čeština dle ČSN 01 6910 (nezlomitelné
  mezery, uvozovky „…“ ‚…‘, pomlčky, rozsahy, `…`, `×`, tisíce úzkou mezerou),
  němčina dle DIN 5008 (z. B. s vázanými mezerami, Nr./S./Abb. k číslu,
  Gedankenstrich, DIN datum, Inhalt/Abb.), angličtina s em dash dle Chicago,
  “uvozovkami” a apostrofy (`don't` → `don’t`); kódu, matematiky a URL se
  nedotýká
- **dělení slov** měkkými rozdělovníky (vzory cs/de/en embedované)
- **tiskové CSS** — A4, orphans/widows, opakování hlavičky tabulky přes
  stránky, nedělené figury, zalamování dlouhých řádků kódu s visutou
  indentací, URL za externími odkazy
- **obrázky** — cesty relativně ke vstupu, kopie `<slug>-<hash>`, úklid řízený
  manifestem (nikdy se nemaže nic cizího), `--fetch` s diskovou cache
- **tmavý režim a velikost písma bez JavaScriptu** — tmavý režim dle
  systému + ruční CSS přepínač ◐ (tisk vždy světlý); přepínač A/A/A volí
  menší/výchozí/větší písmo pro obrazovku i tisk (samotná tlačítka se
  netisknou)
- **template packy** — externí branding (logo, barvy, fonty, patička firmy)
  ze složky za běhu, bez rekompilace

## Front matter (volitelný)

```markdown
---
title: Název dokumentu
author: Jméno Autora
date: 2026-09-02
version: "1.2"
lang: cs
---
```

`title` vytvoří hlavičku dokumentu, ISO datum se česky sází jako
`2. 9. 2026`, `version` se vypíše v podtitulku („verze 1.2“ — pište
v uvozovkách, holé `1.20` by YAML zkrátil na 1.2).

## Konfigurace (`mdprint.toml` vedle vstupu, nebo `--config`)

Viz okomentovaný příklad v [README.md](README.md#configuration) — klíče:
`lang`, `template`, `[paper]` (size, margin), `[figures]` (prefix,
numbering), `[fonts]` (serif, sans, mono), `[math]` (inline, display —
velikost matematiky v em, rozsah 0,5–2,0; zmenšování širokých vzorců
pro tisk se přizpůsobí), `[company]` (patička brandované šablony).
CLI přepínače konfiguraci přebíjejí.

## Template packy

Vlastní vzhled bez rekompilace — složka předaná přes `--template`:

```
muj-pack/
├─ pack.toml           # povinný manifest
├─ template.css        # povinné: CSS vrstva vkládaná ZA vestavěné styly
├─ logo-light.png      # volitelné: logo pro světlý režim a tisk
├─ logo-dark.png       # volitelné: logo pro tmavý režim
├─ fonts/*.woff2       # volitelné: fonty deklarované v pack.toml
└─ icons/*.svg|png     # volitelné: ikony odkazů deklarovaných v pack.toml
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

[[links]]              # odkazy v patičce (opakovatelné)
label = "GitHub"
url = "https://github.com/uzivatel"
icon = "icons/github.svg"  # volitelné; SVG/PNG uvnitř packu, inlinuje se —
                           # monochromatická ikona se v tmavém režimu invertuje
```

Aktivní pack vloží `<header class="brand-header">` (loga + název) a
`<footer class="brand-footer">` s údaji `[company]` a odkazy `[[links]]`;
vzhled řídí `template.css` (třídy `.brand-*`, přepis tokenů `--c-*`,
`--font-*`). Výstup zůstává jeden HTML soubor. Startovní pack k forknutí:
**[mdprint-pack-starter](https://github.com/filip-cokesh/mdprint-pack-starter)**.

## Build ze zdrojáků

```
cargo build --release
cargo test
```

Čistý Rust bez C závislostí (`syntect` s feature `default-fancy`); MSRV 1.88.

## Bezpečnost

mdprint je build nástroj pro **vlastní dokumenty** a svým vstupům věří:
raw HTML z Markdownu projde do výstupu, obrázkové cesty (i absolutní a `..`)
se čtou z disku a kopírují k výstupu, template pack je fakticky kód (jeho
CSS a fonty skončí v každé stránce). Nezpracovávejte nedůvěryhodný Markdown,
konfiguraci ani packy — hlášení zranitelností viz [SECURITY.md](SECURITY.md).

## Licence

[MIT](LICENSE-MIT) nebo [Apache-2.0](LICENSE-APACHE), dle vaší volby.
Fonty a KaTeX mají vlastní licence — [THIRD-PARTY-LICENSES](THIRD-PARTY-LICENSES).

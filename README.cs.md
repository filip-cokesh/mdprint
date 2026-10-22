<img src="assets/logo/mdprint-logo.svg" width="96" align="left" alt="logo mdprint">

# mdprint

**Markdown → tiskově kvalitní, samostatné HTML. Jedna statická binárka.**

<br clear="left">

*English documentation: [README.md](README.md)*

**Živá dema** — skutečné dokumenty z `mdprint --toc --lof --lot`
s číslovanými rovnicemi a křížovými odkazy, tabulkami s popiskem, kódem,
obrázky, typografií a tmavým režimem:
[čeština](https://filip-cokesh.github.io/mdprint/) ·
[English](https://filip-cokesh.github.io/mdprint/en.html) ·
[Deutsch](https://filip-cokesh.github.io/mdprint/de.html).
Zkuste i tisk (Ctrl+P), nebo rovnou **vytištěná PDF** s číslováním stránek
a obsahem, seznamem obrázků a tabulek s čísly stran
(`mdprint --toc-pages --lof --lot --pdf=all`):
[CS](https://filip-cokesh.github.io/mdprint/doc-cs-normal.pdf) ·
[EN](https://filip-cokesh.github.io/mdprint/doc-en-normal.pdf) ·
[DE](https://filip-cokesh.github.io/mdprint/doc-de-normal.pdf)
(i v malém a velkém písmu, odkazy jsou na demostránkách).

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

- **jedna statická binárka** — žádné runtime závislosti (Chrome/Edge jen
  pro volitelné `--pdf` a `--toc-pages`), žádný JavaScript ve výstupu, fonty (Libertinus, JetBrains Mono) embedované jako WOFF2
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
- **tiskové CSS** — A4, číslo stránky „3 / 8“ v patičce, orphans/widows,
  opakování hlavičky tabulky přes stránky, nedělené figury, zalamování
  dlouhých řádků kódu s visutou indentací, URL za externími odkazy
- **čísla stran v obsahu** (`--toc-pages`) — mdprint dokument vysází
  v headless Chrome/Edge ve všech třech velikostech písma, z PDF přečte
  stránku každého nadpisu a čísla zapíše do obsahu; tisk použije sadu
  podle zvolené velikosti písma
- **export do PDF** (`--pdf`, `--pdf=all`) přes stejný headless prohlížeč
- **číslování rovnic a křížové odkazy** LaTeXovou syntaxí — `\label{eq:x}`
  v blokové rovnici, v textu `\eqref{eq:x}` / `\ref{x}` / `\autoref{x}`;
  labely obrázků (`{#fig:x}`) a tabulek s popiskem (`Tabulka: … {#tbl:x}`);
  čísla doplní mdprint při buildu, autor je nikdy nepíše ručně — viz
  [Křížové odkazy a seznamy](#křížové-odkazy-a-seznamy)
- **seznam obrázků a seznam tabulek** (`--lof`, `--lot`) na konci
  dokumentu, s `--toc-pages` i s čísly stran
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
font_size: normal     # small | normal | large — výchozí A/A/A i velikost pro --pdf
page_numbers: true    # číslo stránky v tisku (výchozí zapnuto)
toc_pages: false      # čísla stran v obsahu (zapne i obsah)
lof: false            # seznam obrázků na konci dokumentu
lot: false            # seznam tabulek na konci dokumentu
---
```

`title` vytvoří hlavičku dokumentu, ISO datum se česky sází jako
`2. 9. 2026`, `version` se vypíše v podtitulku („verze 1.2“ — pište
v uvozovkách, holé `1.20` by YAML zkrátil na 1.2).

## Konfigurace (`mdprint.toml` vedle vstupu, nebo `--config`)

Viz okomentovaný příklad v [README.md](README.md#configuration) — klíče:
`lang`, `template`, `[paper]` (size, margin), `[figures]` (prefix,
numbering, list), `[tables]` (prefix, list), `[equations]` (numbering =
labeled | all | none), `[fonts]` (serif, sans, mono), `[math]` (inline,
display — velikost matematiky v em, rozsah 0,5–2,0; zmenšování širokých
vzorců pro tisk se přizpůsobí), `[print]` (page_numbers, toc_pages,
font_size, browser), `[company]` (patička brandované šablony).
CLI přepínače konfiguraci přebíjejí.

## Křížové odkazy a seznamy

Markdown rovnice číslovat neumí a KaTeX zná jen ruční `\tag`. mdprint
proto při buildu vyhodnotí LaTeXové labely — čísla autor nikdy nepíše:

```markdown
$$ w_{\max} = \frac{5 q L^4}{384 E I} \label{eq:pruhyb} $$

Vztah \eqref{eq:pruhyb} plyne z … (viz \autoref{fig:nosnik} a \autoref{tbl:data}).

![Statické schéma nosníku](nosnik.png){#fig:nosnik}

Tabulka: Vstupní data {#tbl:data}

| L | q |
|---|---|
```

- **Rovnice:** `\label{…}` kdekoli v blokové rovnici (`$$…$$`); label se
  odstraní a doplní se `\tag{N}`, který KaTeX vysází vpravo. Ve výchozím
  stavu se číslují jen rovnice s labelem (jako `equation` vs. `equation*`
  v LaTeXu); `[equations] numbering = "all"` očísluje všechny blokové
  rovnice, `\notag` / `\nonumber` číslo potlačí. Rovnice s vlastním `\tag`
  si ho ponechá a odkazy zobrazí jeho text. Číslování je průběžné celým
  dokumentem (1, 2, 3 …).
- **Obrázky:** `{#fig:x}` hned za obrázkem (jméno labelu je libovolné).
  Obrázek bez labelu dostane kotvu `fig-N`.
- **Tabulky:** odstavec začínající `Tabulka:`, `Table:` nebo `Tabelle:`
  (přijímá se v každém jazyce) bezprostředně před tabulkou je její popisek,
  sází se nad tabulkou jako „Tab. 1: …“; `{#tbl:x}` na konci je label.
  Tabulka bez popisku se nečísluje a v seznamu není.
- **Odkazy** v textu (mimo matematiku): `\ref{x}` → holé číslo,
  `\eqref{eq:x}` → „(3)“, `\autoref{x}` → lokalizovaný prefix
  s nezlomitelnou mezerou: *rovnice (3)*, *Obr. 2*, *Tab. 1* (en: *Eq. (3)*,
  *Fig. 2*, *Table 1*; de: *Gl. (3)*, *Abb. 2*, *Tab. 1*). Vše jsou odkazy.
  Neznámý nebo duplicitní label, stejně jako `\label` v řádkové matematice,
  je chyba buildu s číslem řádku. Label: písmena, číslice a `: _ . -`,
  začíná písmenem.
- **Seznamy:** `--lof` / `--lot` (front matter `lof: true`, `lot: true`;
  `[figures] list`, `[tables] list`) připojí za tělo dokumentu *Seznam
  obrázků* / *Seznam tabulek* (*List of Figures* / *List of Tables*,
  *Abbildungsverzeichnis* / *Tabellenverzeichnis*) — v tisku na nové
  stránce. S `--toc-pages` dostanou čísla stran ze stejného měření jako obsah.

## Číslování stránek a čísla v obsahu

- Číslo stránky v patičce je CSS margin box `@page`: tisknou ho Chrome
  a Edge 131+, Safari a Firefox ho zatím vynechají (nic se nerozbije).
  Vypnutí: `--no-page-numbers` nebo `page_numbers = false`.
- Čísla v obsahu (`--toc-pages`) se měří, ne odhadují: tři headless rendery
  (jeden na velikost písma) trvají pár sekund. Jsou přesná pro tisk
  z Chrome/Edge s formátem a okraji dokumentu při měřítku 100 %; jiný
  prohlížeč nebo změna měřítka v dialogu je může posunout. Výstup `--pdf`
  sedí vždy.
- V dialogu tisku vypněte *Záhlaví a zápatí* prohlížeče — Chrome pak
  patičku přenechá mdprintu.
- Prohlížeč se hledá automaticky (Chrome, Edge, Chromium), jinak
  `--browser <cesta>` nebo proměnná `MDPRINT_BROWSER`. Bez něj
  `--toc-pages` jen varuje a obsah zůstane bez čísel; `--pdf` skončí chybou.

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

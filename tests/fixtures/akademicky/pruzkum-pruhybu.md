---
title: Průhyb prostě podepřeného nosníku za rovnoměrného zatížení
author: Filip Hokeš
date: 2026-09-03
version: "1.1"
lang: cs
---

## Úvod

Tato studie se zabývá průhybem prostě podepřeného nosníku o rozpětí $L$,
zatíženého rovnoměrným spojitým zatížením $q$. Cílem je porovnat uzavřené
analytické řešení s numerickým řešením metodou sítí[^sit] a ukázat, za jakých
podmínek vystačíme s Bernoulliho–Navierovou hypotézou. V šedesátých letech
20. století se k výpočtu používaly tabulky, např. dle prof. Šmiřáka;
dnes výpočet zvládne i skript o desítkách řádků.

[^sit]: Metoda sítí (metoda konečných diferencí) — derivace v diferenciální
    rovnici se nahradí diferenčními podíly na pravidelné síti uzlů. Nezaměňovat
    s metodou konečných prvků, která hledá přibližné řešení ve slabé formulaci
    nad bázovými funkcemi prvků.

Maximální průhyb uprostřed rozpětí je dán vztahem

$$w_{\max} = \frac{5\,q\,L^4}{384\,E\,I},$$

kde $E$ je modul pružnosti materiálu a $I$ moment setrvačnosti průřezu.
Pro ocel je typicky $E = 210\,\mathrm{GPa}$; u dřeva počítáme s hodnotou
řádově desetkrát nižší. Poměrný průhyb $w_{\max}/L$ nemá dle normy
překročit hodnotu 1/250.

Uvedený vztah plyne z Bernoulliho–Navierovy hypotézy, která zanedbává
smykovou složku průhybu. O její velikosti nerozhoduje štíhlost sama o sobě,
ale **poměr ohybové a smykové tuhosti**: pro rovnoměrné zatížení platí

$$\frac{w_{\mathrm{V}}}{w_{\mathrm{M}}} = \frac{q L^2 / (8\,G\,A_{\mathrm{s}})}
{5\,q\,L^4 / (384\,E\,I)} = 9{,}6\,\frac{E\,I}{G\,A_{\mathrm{s}}\,L^2},$$

kde $G$ je modul pružnosti ve smyku a $A_{\mathrm{s}}$ smyková plocha průřezu.
Teprve u homogenního obdélníkového průřezu se výraz zjednoduší na
$\approx 2{,}5\,(h/L)^2$, takže pro $h/L = 1/20$ činí smyková složka asi
0,6 % — odtud pochází zažité pravidlo o štíhlosti. U sendvičových,
tenkostěnných či kompozitních průřezů s nízkým $G\,A_{\mathrm{s}}$ však může
být smyk významný i u velmi štíhlého nosníku a je namístě Timoshenkova teorie.

## Vstupní data a parametrická studie

Následující tabulka shrnuje výsledky parametrické studie pro 36 kombinací
rozpětí a zatížení. Tabulka záměrně přesahuje jednu stránku, aby ověřila
opakování hlavičky při tisku.

| č. | L [m] | q [kN/m] | E [GPa] | I [10⁻⁶ m⁴] | w_max [mm] | w/L [–] |
|---:|------:|---------:|--------:|------------:|-----------:|--------:|
| 1 | 3,0 | 5,0 | 210 | 83,3 | 0,60 | 1/4970 |
| 2 | 3,0 | 10,0 | 210 | 83,3 | 1,21 | 1/2485 |
| 3 | 3,0 | 15,0 | 210 | 83,3 | 1,81 | 1/1657 |
| 4 | 3,5 | 5,0 | 210 | 83,3 | 1,12 | 1/3133 |
| 5 | 3,5 | 10,0 | 210 | 83,3 | 2,23 | 1/1567 |
| 6 | 3,5 | 15,0 | 210 | 83,3 | 3,35 | 1/1044 |
| 7 | 4,0 | 5,0 | 210 | 83,3 | 1,90 | 1/2100 |
| 8 | 4,0 | 10,0 | 210 | 83,3 | 3,81 | 1/1050 |
| 9 | 4,0 | 15,0 | 210 | 83,3 | 5,71 | 1/700 |
| 10 | 4,5 | 5,0 | 210 | 83,3 | 3,05 | 1/1475 |
| 11 | 4,5 | 10,0 | 210 | 83,3 | 6,10 | 1/738 |
| 12 | 4,5 | 15,0 | 210 | 83,3 | 9,15 | 1/492 |
| 13 | 5,0 | 5,0 | 210 | 83,3 | 4,65 | 1/1075 |
| 14 | 5,0 | 10,0 | 210 | 83,3 | 9,30 | 1/538 |
| 15 | 5,0 | 15,0 | 210 | 83,3 | 13,95 | 1/358 |
| 16 | 5,5 | 5,0 | 210 | 83,3 | 6,81 | 1/808 |
| 17 | 5,5 | 10,0 | 210 | 83,3 | 13,62 | 1/404 |
| 18 | 5,5 | 15,0 | 210 | 83,3 | 20,43 | 1/269 |
| 19 | 6,0 | 5,0 | 210 | 83,3 | 9,64 | 1/622 |
| 20 | 6,0 | 10,0 | 210 | 83,3 | 19,28 | 1/311 |
| 21 | 6,0 | 15,0 | 210 | 83,3 | 28,93 | 1/207 |
| 22 | 6,5 | 5,0 | 210 | 83,3 | 13,29 | 1/489 |
| 23 | 6,5 | 10,0 | 210 | 83,3 | 26,57 | 1/245 |
| 24 | 6,5 | 15,0 | 210 | 83,3 | 39,86 | 1/163 |
| 25 | 7,0 | 5,0 | 210 | 83,3 | 17,86 | 1/392 |
| 26 | 7,0 | 10,0 | 210 | 83,3 | 35,71 | 1/196 |
| 27 | 7,0 | 15,0 | 210 | 83,3 | 53,57 | 1/131 |
| 28 | 7,5 | 5,0 | 210 | 83,3 | 23,54 | 1/319 |
| 29 | 7,5 | 10,0 | 210 | 83,3 | 47,08 | 1/159 |
| 30 | 7,5 | 15,0 | 210 | 83,3 | 70,62 | 1/106 |
| 31 | 8,0 | 5,0 | 210 | 83,3 | 30,48 | 1/262 |
| 32 | 8,0 | 10,0 | 210 | 83,3 | 60,95 | 1/131 |
| 33 | 8,0 | 15,0 | 210 | 83,3 | 91,43 | 1/87 |
| 34 | 8,5 | 5,0 | 210 | 83,3 | 38,84 | 1/219 |
| 35 | 8,5 | 10,0 | 210 | 83,3 | 77,68 | 1/109 |
| 36 | 8,5 | 15,0 | 210 | 83,3 | 116,52 | 1/73 |

Výsledky pro rozpětí nad 8,0 m poměrný průhyb 1/250 zjevně nesplňují
a vyžadují buď vyšší průřez, nebo nadvýšení.

## Výpočetní skript

Analytické řešení ověřuje následující skript. Řádky jsou záměrně dlouhé,
aby prověřily zalamování kódu při tisku.

```python
"""Průhyb prostě podepřeného nosníku: analyticky vs. numericky (metoda sítí)."""
import numpy as np

def analyticke_reseni(L: float, q: float, E: float, I: float, n: int = 101) -> np.ndarray:
    """Vrátí průhybovou čáru w(x) dle vztahu w(x) = q x (L^3 - 2 L x^2 + x^3) / (24 E I)."""
    x = np.linspace(0.0, L, n)
    return q * x * (L**3 - 2.0 * L * x**2 + x**3) / (24.0 * E * I)

def numericke_reseni(L: float, q: float, E: float, I: float, n: int = 101) -> np.ndarray:
    """Řeší E I w'''' = q metodou sítí s okrajovými podmínkami w(0) = w(L) = 0, w''(0) = w''(L) = 0."""
    h = L / (n - 1)
    A = np.zeros((n, n)); b = np.full(n, q * h**4 / (E * I))
    for i in range(2, n - 2):
        A[i, i-2:i+3] = [1.0, -4.0, 6.0, -4.0, 1.0]
    A[0, 0] = A[n-1, n-1] = 1.0; b[0] = b[n-1] = 0.0          # w = 0 v podporách
    A[1, 0:3] = [1.0, -2.0, 1.0]; b[1] = 0.0                   # w'' = 0 (kloub vlevo)
    A[n-2, n-3:n] = [1.0, -2.0, 1.0]; b[n-2] = 0.0             # w'' = 0 (kloub vpravo)
    return np.linalg.solve(A, b)

if __name__ == "__main__":
    L, q, E, I = 6.0, 10_000.0, 210e9, 83.3e-6
    w_a = analyticke_reseni(L, q, E, I)
    w_n = numericke_reseni(L, q, E, I)
    print(f"analyticky w_max = {1000 * w_a.max():.3f} mm, numericky w_max = {1000 * w_n.max():.3f} mm")
    print(f"relativní odchylka = {abs(w_n.max() - w_a.max()) / w_a.max():.2e}")
```

## Grafické výsledky

Průhybové čáry obou řešení jsou vykresleny na následujícím obrázku.

![Průhybová čára — porovnání analytického a numerického řešení](grafy/průběh.png)

Statické schéma úlohy se stejnojmenným souborem z jiné složky:

![Statické schéma prostě podepřeného nosníku](schemata/průběh.png)

## Zdroj vs. výsledek

Tento dokument vznikl z prostého Markdownu. Následující blok je doslovný
zdrojový text (uvnitř bloku kódu se typografická pravidla záměrně
neuplatňují):

```markdown
Nosník o rozpětí 6 m nese zatížení 10 kN/m; průhyb w_max = 9,65 mm
"vyhoví" i pro mezní stav 1/250 - viz obr. 1. Teplotní rozsah 10-20 °C,
plocha 40x60 mm, 1 000 000 cyklů, posudek dle ČSN EN 1990 s. 12.
Vzorec $w_{\max} = 5qL^4/(384EI)$ i cesta C:\data\vstup.md zůstávají
netknuté.
```

A takto tentýž text vysází mdprint:

Nosník o rozpětí 6 m nese zatížení 10 kN/m; průhyb w_max = 9,65 mm
"vyhoví" i pro mezní stav 1/250 - viz obr. 1. Teplotní rozsah 10-20 °C,
plocha 40x60 mm, 1 000 000 cyklů, posudek dle ČSN EN 1990 s. 12.
Vzorec $w_{\max} = 5qL^4/(384EI)$ i cesta `C:\data\vstup.md` zůstávají
netknuté.

Rozdíl je v detailech, které rozhodují o čitelnosti sazby: české uvozovky,
pomlčka místo spojovníku, en dash v rozsahu, znak násobení, úzká mezera
v tisícových skupinách a nezlomitelné mezery za jednopísmennými předložkami,
zkratkami i mezi číslem a jednotkou. Matematika se vysází, kód a cesty
zůstanou beze změny.

## Závěr

Numerické řešení se od analytického liší o méně než 0,01 %, což potvrzuje
správnost obou přístupů. Inline vztah $w(x) = \frac{q x (L^3 - 2Lx^2 + x^3)}{24 E I}$
lze proto bez obav používat pro rychlé posouzení[^posudek]. Definiční obor:

Mezní stav použitelnosti

: Stav, při jehož překročení přestává konstrukce plnit provozní požadavky,
  ačkoli nedochází k jejímu porušení.

[^posudek]: Posudek dle ČSN EN 1990 vyžaduje kombinace zatížení, které tento
    text pro stručnost vynechává.

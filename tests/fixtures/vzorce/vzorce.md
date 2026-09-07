---
title: Repro — dlouhé vzorce v tisku
author: Filip Hokeš
date: 2026-09-06
lang: cs
---

## Dlouhý display vzorec (jeden base, nelámatelný)

Po dosazení ($E = 210 \cdot 10^9$ Pa, síly v N, délky v m):

$$\Delta l = \frac{1}{210 \cdot 10^9} \left( \frac{-300 \cdot 10^3 \cdot 3{,}4}{7{,}38 \cdot 10^{-3}} + \frac{-700 \cdot 10^3 \cdot 3{,}6}{7{,}38 \cdot 10^{-3}} + \frac{-1100 \cdot 10^3 \cdot 3{,}6}{1{,}016 \cdot 10^{-2}} + \frac{-1500 \cdot 10^3 \cdot 3{,}6}{1{,}016 \cdot 10^{-2}} + \frac{-1900 \cdot 10^3 \cdot 3{,}6}{1{,}414 \cdot 10^{-2}} + \frac{-2400 \cdot 10^3 \cdot 3{,}8}{1{,}414 \cdot 10^{-2}} \right)$$

## Display s top-level operátory (lámatelný)

$$\Delta l = \frac{N_1 l_1}{E A_1} + \frac{N_2 l_2}{E A_2} + \frac{N_3 l_3}{E A_3} + \frac{N_4 l_4}{E A_4} + \frac{N_5 l_5}{E A_5} + \frac{N_6 l_6}{E A_6} = \frac{1}{E} \sum_{i=1}^{6} \frac{N_i l_i}{A_i} = -12{,}4 \text{ mm}$$

## Krátký display (nesmí se zmenšit)

$$w_{\max} = \frac{5\,q\,L^4}{384\,E\,I}$$

## Dlouhý inline vzorec

Pro kontrolu lze vyčíslit i největší napětí, které vzniká v nejnižším
úseku: $\sigma_6 = N_6/A_6 = -2400 \cdot 10^3/(1{,}414 \cdot 10^{-2}) \approx -170 \text{ MPa}$,
tedy hodnotu bezpečně pod mezí kluzu běžných konstrukčních ocelí. Krátké
inline $E = 210\,\mathrm{GPa}$ zůstává beze změny.

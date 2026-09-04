---
title: Typesetting technical English with mdprint
author: Filip Hokeš
date: 2026-09-04
version: "1.0"
lang: en
---

<nav style="text-align:right;font-size:0.85em"><a href="index.html">Čeština</a> · <a href="en.html">English</a> · <a href="de.html">Deutsch</a></nav>

## Why bother with typography

Plain Markdown gives you straight quotes, hyphens where dashes belong and
no control over line breaks - mdprint fixes that at build time. This page
was generated with `mdprint --toc --lang en` and everything you see - the
em dashes, the quotes, the math below - came out of a plain text file.

## What English mode does

The rules "just work" without any configuration: contractions like don't
and isn't get a proper apostrophe, ranges such as pages 10-20 or the years
1918-1938 receive an en dash, dimensions like 40x60 mm use a real
multiplication sign, and a value never loses its unit at a line break -
10 kg, 25 °C or 230 V stay together. Sentence-level dashes - like these -
become em dashes, the American way. Hexadecimal literals such as 0x1F are
left alone, and so is code: `don't touch - this` stays raw.

## Mathematics

The maximum deflection of a simply supported beam under uniform load is

$$w_{\max} = \frac{5\,q\,L^4}{384\,E\,I},$$

where $E$ denotes Young's modulus and $I$ the second moment of area. For
steel, $E = 210\,\mathrm{GPa}$; codes usually limit the relative deflection
$w_{\max}/L$ to 1/250.

## Code

```rust
fn deflection(q: f64, l: f64, e: f64, i: f64) -> f64 {
    // w_max = 5 q L^4 / (384 E I)
    5.0 * q * l.powi(4) / (384.0 * e * i)
}
```

## Source vs. result

The block below is the literal source of the paragraph that follows it:

```markdown
She said "the span is 6 m" - and she's right: pages 10-20 of the report
list a 40x60 mm section loaded to 25 °C... See section 3 for details.
```

She said "the span is 6 m" - and she's right: pages 10-20 of the report
list a 40x60 mm section loaded to 25 °C... See section 3 for details.

Compare the quotes, the dash, the range, the multiplication sign and the
ellipsis - five silent corrections in two lines.

# Quick Syntax

NODX should stay readable while carrying more structure than Markdown. Prefer
these local shorthands first; use `::style` only for reusable rules, media
queries, or theme-level overrides.

## Inline text

```nodx
**strong** and *emphasis*
~~deleted~~
==highlight=={bg="#ffe08a" color="#111827"}
H~2~O and x^2^
[[Approved]].status.success
```

The highlight suffix uses the same attribute grammar as spans, so classes,
`title`, `lang`, `dir`, and safe style shorthands work without a style block.

## Page Background

Use front matter for document-wide color and background image:

```nodx
---
theme: web
page:
  bg: "#101827"
  color: "#f8fafc"
  background: "assets/background.png"
---
```

`background` is package-relative and goes through the normal safe asset policy.
Remote image loading remains host-controlled.

For a page-like region inside a document:

```nodx
:::page {bg="#ffffff" background="assets/page-bg.png" pad="2rem"}
# Page section

Content.
:::
```

## Layout Without Style Blocks

```nodx
:::grid {columns="repeat(3,1fr)" gap="1rem"}
::frame {bg="#f8fafc" border="1px solid #d1d5db" pad="1rem"}
First
::
::frame {bg="#fff7ed" pad="1rem"}
Second
::
:::
```

Useful shorthands:

| Write | Meaning |
|---|---|
| `bg="#f8fafc"` | `background-color` |
| `color="#111827"` | text color |
| `border="1px solid #ddd"` | border |
| `radius="6px"` | border radius |
| `pad="1rem"` | padding |
| `margin="1rem 0"` or `m="1rem 0"` | margin |
| `gap="1rem"` | grid/flex gap |
| `columns="repeat(3,1fr)"` | grid columns |
| `text-align="center"` | text alignment |

## Tables

Pipe tables stay compact but can carry structural cell attributes:

```nodx
| Item | Amount |
| :--- | ---: |
| {colspan=2 align="center"} Total | |
```

Use block-form tables when cells need nested blocks:

```nodx
:::table {caption="Quarterly revenue"}
:::row
:::cell {header="true" colspan=2 align="center"}
Total
:::
:::
:::
```

## When To Use `::style`

Use `::style` for:

- rules shared by many nodes;
- dark/light variants with `@media`;
- print-specific behavior;
- selectors that cannot be expressed as local attributes.

Keep one-off visual choices in attributes so the body stays close to the
content.

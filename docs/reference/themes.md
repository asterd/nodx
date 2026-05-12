# Themes and styling

NODX renders to plain, semantic HTML and lets a stylesheet decide what
"plain" looks like. There are six built-in themes you can pick from front
matter, and a small surface of design tokens that survive every theme
switch.

## Built-in themes

| Theme | Description | When |
|---|---|---|
| `none` | No stylesheet at all. Bare HTML output. | Embedding NODX HTML into a page that already has its own CSS. |
| `plain` | Conservative typographic defaults, no colors. | E-mail, plain web pages, simple documents. |
| `base` | Sensible reading defaults, modest accents. | The unopinionated default for documents that do not declare a theme. |
| `web` | Polished web reading experience. | Marketing pages, blog posts, anything that wants to look modern in a browser. |
| `print` | Paged layout for print and PDF. | Reports, statements, anything destined for paper. |
| `presentation` | Slide-oriented layout, big headings, controlled white space. | `.pptx` exports and projector-friendly viewing. |
| `docs` | Navigable documentation shell with left nav and right outline. | This documentation site. Reference manuals. |

The theme is declared in front matter:

```nodx
---
title: My document
theme: web
---
```

A document without `theme:` gets `base`. A document that uses
`layout: docs` requests the documentation shell even if a different
`theme:` is declared — `layout:` controls structural rendering, `theme:`
controls visual rendering. The two compose.

## Design tokens

Override these CSS custom properties first; they are how every built-in
theme is built, and they are stable across releases.

```css
:root {
  --nodx-color-text:   #1f2937;   /* body text */
  --nodx-color-muted:  #4b5563;   /* secondary text */
  --nodx-color-bg:     #ffffff;   /* page background */
  --nodx-color-primary: #0f766e;  /* links, accents */
  --nodx-color-accent:  #b91c1c;  /* highlights, callouts */
  --nodx-color-rule:    #e5e7eb;  /* dividers, table borders */

  --nodx-font-body:    system-ui, sans-serif;
  --nodx-font-heading: var(--nodx-font-body);
  --nodx-font-mono:    ui-monospace, monospace;

  --nodx-line-height:  1.6;
  --nodx-block-gap:    1rem;
  --nodx-page-margin:  22mm;
}
```

Override them in a `::style` block:

```nodx
::style
:root {
  --nodx-color-primary: #1d4ed8;
  --nodx-font-heading: Georgia, serif;
}
::
```

That is the most portable way to brand a document. Everything else falls
back to the same defaults.

## Selectors that survive the audit

The NODS safe subset accepts standard CSS selectors but rejects anything
that looks scripted or networked. Forbidden constructs:

- `expression(…)`, `javascript:`, `vbscript:`, `url(http…)`, `@import url`
- `position: fixed/absolute/sticky` (no overlays)
- `:hover`, `:focus`, `:focus-within` (no interactive states)
- `behavior:` (legacy IE backdoor)
- `data:` urls outside the image MIME types
- font face declarations referencing remote URLs

Rules containing any of these raise `NODX-E027` and are *removed* from the
rendered HTML. Adjacent safe rules still ship.

For the full audit code, see
[`crates/nodx-style/src/lib.rs`](../../crates/nodx-style/src/lib.rs).

## Component selectors

Renderer-stable HTML for every block:

| NODX component | HTML | Recommended selectors |
|---|---|---|
| `heading` | `<h1>` … `<h6>` | `h1`, `h2`, `h3`, `h4`, `h5`, `h6` |
| `paragraph` | `<p>` | `p` |
| `section` | `<section>` | `section`, `section.<class>` |
| `note` | `<aside>` | `aside`, `aside[title]` |
| `quote` | `<blockquote>` | `blockquote` |
| `list` | `<ul>` or `<ol>` | `ul`, `ol`, `li` |
| `item` | `<li>` | `li` |
| `code`, `pre` | `<pre><code>` | `pre`, `pre code` |
| `math` | `<pre class="math">` | `pre.math` |
| inline math | `<code class="math-inline">` | `.math-inline` |
| `table` | `<table>` | `table`, `thead`, `tr`, `th`, `td` |
| `figure` | `<figure>` | `figure` |
| `caption` | `<figcaption>` | `figcaption` |
| `image` | `<img>` or `<span class="nodx-blocked-image">` | `img`, `.nodx-blocked-image` |
| `form` | `<dl>` | `dl`, `dt`, `dd` |
| `toc` | `<nav>` | `nav`, `nav ol`, `nav li[data-level="2"]` etc. |
| `pagebreak` | `<hr class="pagebreak">` | `.pagebreak` |
| `bibliography` | `<ol>` | `ol li` |
| `speaker-notes` | `<aside class="speaker-notes">` | `.speaker-notes` |
| custom component | `<section class="nodx-component nodx-component--fallback" data-component="name">` | `.nodx-component`, `[data-component="name"]` |
| blocked link | `<a class="nodx-blocked-link">` | `.nodx-blocked-link` |
| mention | `<span class="mention">` | `.mention` |
| inline span | `<span>` | `span`, `span[lang]` |

## Package-level themes

Packages can ship `.nods` stylesheets and reference them from the
manifest:

```yaml
themes:
  - path: themes/brand.nods
```

The package reader applies them after the chosen theme, in declared
order. The same NODS safety subset applies.

## Front-matter components

Renderers resolve local component templates declared inline:

```nodx
---
components:
  - name: approval-card
    template: |
      :::note {class="nodx-component nodx-component--approval-card"}
      ## {{title}}
      {{children}}
      :::
    style: |
      .nodx-component--approval-card {
        border-left: 5px solid var(--nodx-color-primary);
      }
---
```

Unknown components still fall back to the safe section wrapper described
above. Authored content is preserved verbatim.

## What "safe styling" buys you

- Pasting an unknown stylesheet from an untrusted source cannot inject
  scripts.
- A document survives being moved between themes — the design tokens are
  what survives, not pixel coordinates.
- Tools that don't render CSS (TUI viewer, NCP exporter) still see the
  same semantic structure that the styled HTML shows.

The contract is intentionally narrower than full CSS. NODX is not a
graphic design tool.

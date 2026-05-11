# NODX Theming And CSS Extension

NODX renderers emit semantic HTML with stable tags, classes, attributes, and
theme tokens. Extend themes with safe `::style` blocks or package-local `.nods`
stylesheets. Keep overrides inside the NODS safe subset: no scripts, remote
imports, interactive pseudo-classes, absolute positioning, or unsafe URLs.

## Theme Sources

The standard theme sources are committed as `.nods` files:

| Theme | Source |
|---|---|
| `none` | [`docs/themes/none.nods`](./themes/none.nods) |
| `plain` | [`docs/themes/plain.nods`](./themes/plain.nods) |
| `base` | [`docs/themes/base.nods`](./themes/base.nods) |
| `web` | [`docs/themes/web.nods`](./themes/web.nods) |
| `print` | [`docs/themes/print.nods`](./themes/print.nods) |
| `presentation` | [`docs/themes/presentation.nods`](./themes/presentation.nods) |

## Design Tokens

Override these tokens first; they are the most portable theme extension points.

```nodx
:::style
:root {
  --nodx-color-text: #1f2937;
  --nodx-color-muted: #4b5563;
  --nodx-color-primary: #0f766e;
  --nodx-color-accent: #b91c1c;
  --nodx-font-body: system-ui, sans-serif;
  --nodx-font-heading: var(--nodx-font-body);
  --nodx-font-mono: ui-monospace, monospace;
  --nodx-page-margin: 22mm;
  --nodx-line-height: 1.6;
  --nodx-block-gap: 1rem;
}
:::style
```

## Component Selectors

Use these selectors to overload renderer CSS for each supported NODX component.

| NODX component | HTML emitted by renderer | Recommended selectors |
|---|---|---|
| `heading` | `h1` through `h6` | `h1`, `h2`, `h3`, `h4`, `h5`, `h6`, `[id]` |
| `paragraph` | `p` | `p` |
| `section` | `section` | `section`, `section.<class>` |
| `note` | `aside` | `aside`, `aside[title]`, `.note` if authored as a class |
| `quote` | `blockquote` | `blockquote` |
| `list` | `ul` or `ol` | `ul`, `ol`, `li` |
| `item` | `li` | `li` |
| `code` / `pre` | `pre > code` | `pre`, `pre code`, `code` |
| `math` | `pre.math` | `pre.math` |
| inline math | `code.math-inline` | `.math-inline` |
| `style` | `style` | not styled directly |
| `table` | `table` | `table`, `thead`, `tr`, `th`, `td` |
| `row` | `tr` | `tr` |
| `cell` | `td` or `th` | `td`, `th` |
| `figure` | `figure` | `figure` |
| `caption` | `figcaption` | `figcaption` |
| `image` | `img` or `span.nodx-blocked-image` | `img`, `.nodx-blocked-image` |
| `form` | `dl` | `dl`, `dt`, `dd` |
| `field` | `div > dt + dd` | `dt`, `dd`, `dl > div` |
| `toc` | `nav` | `nav`, `nav ol`, `nav li[data-level="2"]` through `nav li[data-level="6"]` |
| `pagebreak` | `hr.pagebreak` | `.pagebreak` |
| `media`, `embed`, `include` | `figure > .media-fallback` | `.media-fallback`, `figure` |
| `bibliography` | `ol` | `ol`, `ol li` |
| `citation-entry` | `li` | `li` |
| `speaker-notes` | `aside.speaker-notes` | `.speaker-notes` |
| custom component with `-` | `section.nodx-component.nodx-component--fallback[data-component="name"]` | `.nodx-component`, `.nodx-component__title`, `[data-component="approval-card"]` |
| blocked unsafe link | `a.nodx-blocked-link` | `.nodx-blocked-link` |
| mention | `span.mention` | `.mention` |
| inline span | `span` plus safe attrs/classes | `span`, `span[lang]`, `.your-class` |
| variable fallback | `var` | `var` |

## Document-Level Extension Example

```nodx
---
schema: nodx/1.0
theme: web
profiles:
  requires:
    - core
    - style
---

# Branded report #report

:::style
:root {
  --nodx-color-primary: #1d4ed8;
  --nodx-color-accent: #be123c;
  --nodx-font-heading: Georgia, serif;
}

h1 { color: var(--nodx-color-primary); }
aside { border-inline-start-color: var(--nodx-color-accent); }
table { width: 100%; }
.nodx-component[data-component="approval-card"] {
  border-color: var(--nodx-color-primary);
  background: #eff6ff;
}
:::style
```

## Package-Level Extension

Packaged documents can include `.nods` files. The web playground applies all
package `.nods` stylesheets after the selected theme, so they can extend or
override the committed theme sources without changing the document text.

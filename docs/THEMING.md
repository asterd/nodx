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
| `docs` | built into renderers as the navigable documentation layout theme |

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
| inline span | `span` plus safe attrs/classes/styles | `span`, `span[lang]`, `.your-class` |
| variable fallback | `var` | `var` |

## Inline Span Styling

Use `[[text]]` for authored inline spans. It accepts the same attribute block as
legacy `[text]{...}`, plus class suffix sugar immediately after the span.

```nodx
[[Status]]{.pill color="var(--nodx-color-primary)" bg="#ccfbf1" radius="999px" pad="2px 8px"}
[[Important]]{highlight}
[[Approved]].status.success
```

Safe shorthands map into AST `styles`: `color`, `bg`, `border`, `radius`,
`pad`/`padding`, `font`, `weight`, and the `highlight` preset. Unsafe values
containing executable CSS constructs, `url(...)`, breakouts, or declaration
separators are ignored rather than emitted.

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

Packaged documents can include `.nods` files and component templates. Declare
self-contained extensions in `manifest.yaml`:

```yaml
components:
  - path: components/approval-card.nodx
themes:
  - path: themes/docs.nods
```

Component files are `.nodx` templates. Their front matter can declare `name`
and optional `style`; the remaining body is used as the template:

```nodx
---
schema: nodx/1.0
name: approval-card
style: |
  .nodx-component--approval-card {
    border-left: 5px solid var(--nodx-color-primary);
  }
---
:::note {class="nodx-component nodx-component--approval-card"}
## {{title}}
{{children}}
:::
```

Renderers expose package extension APIs so editors, CLIs, and previews can
open a package, apply local components/themes, and render without network
access. Remote libraries should be resolved by the host, verified with
integrity, and opened through the same package path.

## Front Matter Components

Renderers resolve local inline component templates declared in front matter.
Unknown components still fall back to the safe component section.

```nodx
---
schema: nodx/1.0
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

::approval-card {title="Approved"}
Children are rendered at `{{children}}`.
::
```

Template variables read component attributes by name, `attrs.name`, or
`vars.name` from front matter. Core renderers do not fetch component `src`
references; hosts that resolve package-local or remote components should keep
the same fail-closed URL and integrity policy used for assets and styles.

## Documentation Layout

Use `theme: docs` or `layout: docs` for navigable documentation. Renderers emit
a responsive shell with a left document navigation, central content, and a
right section outline derived from the document navigation graph.

```nodx
---
schema: nodx/1.0
title: Product Docs
theme: docs
layout: docs
---

# Product Docs {#product-docs}

## Getting Started {#getting-started}

### Install {#install}
```

The layout is structural renderer output, not just a CSS override. Documents
remain valid NODX and degrade to normal content in renderers that do not opt
into the docs shell.

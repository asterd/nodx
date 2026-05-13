# Block reference

Every NODX block: name, purpose, attributes, HTML output, expected role in
the AST. Implementer-grade detail. For a friendlier introduction, see the
[Syntax tour](../guide/02-syntax-tour.md).

## Naming and shape

A delimited block opens with two or more colons, a name in the
`[A-Za-z][A-Za-z0-9-]*` shape, an optional attribute block, and a matching
close:

```nodx
::name {attrs}
content
::

:::name {attrs}
content with nested
::sub
:::
```

The number of colons must match between open and close. Two-colon blocks
(the *Lite* form) cannot directly contain other two-colon blocks; nest by
using three or more colons.

A close may name the block (`:: note`, with `::note` accepted contextually by
the reference parser) to make mismatches loud. The parser records `NODX-E005`
if the names disagree.

## Built-in container blocks

| Name | HTML | Notes |
|---|---|---|
| `section` | `<section>` | Logical grouping. Common parent for navigation. |
| `note` | `<aside>` | `type` attribute drives styling: `info`, `warning`, `danger`, `success`, `note`. |
| `quote` | `<blockquote>` | Optional `cite` attribute for the source URL. |
| `figure` | `<figure>` | Wraps an `image`, `code`, or other media plus an optional `caption`. |
| `caption` | `<figcaption>` | Only valid inside `figure`. |
| `image` | `<img>` or `<span class="nodx-blocked-image">` | Required `src` and `alt`. Unsafe URLs are blocked, not silently rewritten. |
| `media` | `<figure class="media-fallback">` | Future-facing media block. Renders a fallback for now. |
| `embed` | `<figure class="media-fallback">` | Same fallback shape; reserved. |
| `include` | `<figure class="media-fallback">` | Reserved for include extensions. Core renderer does not fetch. |
| `table` | `<table>` | Either authored as a block or implicitly via pipe-table syntax. |
| `row` | `<tr>` | Direct child of `table`. |
| `cell` | `<td>` or `<th>` | Header cells when `header="true"`. Supports `colspan`, `rowspan`, `align`, `valign`, and `scope`. |
| `grid` | `<div class="nodx-grid">` | Responsive grid layout container. Tune with `columns`, `gap`, `pad`, `bg`, etc. |
| `columns` | `<div class="nodx-columns">` | Multicolumn flow container. Tune with `gap`, `width`, and style blocks. |
| `frame` | `<div class="nodx-frame">` | Bordered/padded frame container for grouped content. |
| `page` | `<div class="nodx-page">` | Page-like region with quick `bg`, `background`, spacing, and frame attributes. |
| `list` | `<ul>` or `<ol>` | `kind` attribute is one of `unordered`, `ordered`, `task`. |
| `item` | `<li>` | Optional `checked="true|false"` for task lists. |
| `form` | `<dl>` | Document-style forms; fields render as `dt`/`dd` pairs. |
| `field` | inside `<dl>` as `<dt>`/`<dd>` | Has `label`, `name`, optional `value`. |
| `toc` | `<nav>` | Generated from heading graph. Honors `depth`, `title`, `role`. |
| `pagebreak` | `<hr class="pagebreak">` | Hints to paged output. No content. |
| `bibliography` | `<ol>` | Wraps citation entries. |
| `citation-entry` | `<li>` | Has `id`, optional `label`. |
| `speaker-notes` | `<aside class="speaker-notes">` | Hidden in display renderers, exposed in presentation/agent modes. |

## Built-in literal blocks

Literal blocks preserve their body verbatim, with no further inline parsing:

| Name | HTML | Use |
|---|---|---|
| `code` | `<pre><code>` | Code snippets. `language` attribute hints the highlighter. |
| `pre` | `<pre>` | Pre-formatted text without code semantics. |
| `math` | `<pre class="math">` | Block math. Body is the raw source. |
| `style` | `<style>` (sanitized) | Inline NODS stylesheet. Audited against the safe subset; forbidden rules raise `NODX-E027` and are dropped. |

## Textual blocks

These hold inline content directly, not nested blocks:

| Name | HTML | Notes |
|---|---|---|
| `heading` | `<h1>` … `<h6>` | `level` attribute carries the depth. Lite form `# Title #id` produces the same AST as `# Title {#id}`. |
| `paragraph` | `<p>` | The default for any line of prose. |
| `cell` | `<td>` or `<th>` | Lives only inside `row`. |
| `item` | `<li>` | Lives only inside `list`. |

## Pipe tables

```nodx
| Metric  | Value |
|---------|------:|
| Revenue |  120K |
```

The header row uses `header="true" scope="col"` on each cell. Column
alignment is encoded as `align="left|center|right"` on the header and data cells
based on the `:` markers in the separator row. Inconsistent column counts
raise `NODX-E025`.

Cells may start with a normal attribute block for compact rich tables:

```nodx
| Item | Amount |
| :--- | ---: |
| {colspan=2 align="center"} Total | |
```

The attribute block is removed from the visible cell text. Explicit cell
attributes override separator-row alignment. `colspan` contributes to the
validated table grid width, so a row with one `colspan=2` cell matches a
two-cell row. Use block-form tables when a cell needs nested paragraphs,
lists, images, or other blocks:

```nodx
:::table {caption="Quarterly revenue"}
:::row
:::cell {header="true" colspan=2 align="center"}
Total
:::
:::
:::
```

## Custom components

A `::component-name` block where `component-name` contains a hyphen is
treated as a custom component. Renderers resolve it in two ways:

1. **Front matter components** — declared inline in `components:`.
2. **Package components** — declared in a `.nodx` package's
   `manifest.yaml` under `components:`.

A component template is a `.nodx` snippet whose body becomes the block's
rendered output. `{{title}}`, `{{children}}`, and any `{{attrs.x}}` placeholders are substituted from the call site.

If no template matches, the renderer emits a *safe fallback* — a
`<section class="nodx-component nodx-component--fallback" data-component="name">`
containing the block's authored content. The fallback always preserves
text; it never executes anything; it does not pull from the network.

## Attribute conventions

Some attributes are reserved across blocks:

| Attribute | Where | Meaning |
|---|---|---|
| `id` | any | Stable anchor. `[A-Za-z][A-Za-z0-9-]*`, ≤ 256 bytes, unique. |
| `class` | any | Space-separated class list. Merged with `.foo` shorthand. |
| `lang` | any | BCP-47 language tag. |
| `dir` | any | `ltr`, `rtl`, or `auto`. |
| `title` | links, headings | Renders as the tooltip / accessible name. |
| `role` | `toc`, custom | ARIA role. The renderer keeps it as-is. |
| `data-*` | any | Pass-through; useful for tool integration. |
| `caption` | `table` | Renders as `<caption>`. `title` is accepted as a table-caption alias by HTML renderers. |
| `colspan` | `cell` | Positive integer column span. Counts toward table grid validation. |
| `rowspan` | `cell` | Positive integer row span. Preserved for renderers. |
| `align` | `table`, `cell` | Horizontal alignment. Pipe tables set this from separator markers. Use `text-align` for CSS-style alignment on other nodes. |
| `valign` | `cell` | Vertical alignment hint: `top`, `middle`, `bottom`, `baseline`. |

All other attributes are block-specific. The renderer escapes every
attribute value into the right HTML context — there is no path by which
an authored attribute can break out of its context.

## Forbidden by construction

- No raw HTML inside a NODX document. There is no `:::html` block and there
  will not be one. If you need a feature, it has to make it into the AST.
- No active content — no scripts, no `on*` handlers, no `javascript:` URLs.
  These are dropped at the URL classifier (`NODX-E020`) or at the style
  auditor (`NODX-E027`).
- No remote `@import`, no `url(…)` in inline styles, no `<style>` content
  that resolves outside the document.

The point is that "unknown component" degrades to "safe fallback section",
not "arbitrary HTML execution".

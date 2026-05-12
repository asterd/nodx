# Syntax tour

A NODX document is plain UTF-8 text. If you know Markdown you already know
80% of the surface. The rest is *named, structured blocks* and *typed
attributes* — the parts Markdown leaves to convention.

## The shape of a document

```nodx
---
title: My document
language: en
---

# Heading

A paragraph of text.

::note {type="info"}
A named block, with attributes.
::
```

Two regions:

1. **Front matter** between the first two `---` lines, in a strict YAML safe
   subset. Optional in Lite form — the parser falls back to safe defaults if
   you skip it.
2. **Body**, made of blocks separated by blank lines.

## Headings

```nodx
# Top heading
## Sub-heading
### And so on

# Heading with a stable id #intro
## Heading with attributes {.lead role="banner"}
```

The id from `#intro` is canonicalized and reachable from links, the table of
contents, and the NCP projection. Always prefer named ids over auto-generated
slugs when you want stable URLs.

## Inline syntax

| You write | You get |
|---|---|
| `**strong**` | bold emphasis |
| `*emphasis*` | italic emphasis |
| `` `code` `` | inline code |
| `==mark==` | highlighted text |
| `~sub~` | subscript |
| `^sup^` | superscript |
| `$$x+1$$` | inline math (raw, not evaluated) |
| `[label](https://example.com)` | safe link |
| `[label](docs/guide.nodx){title="…" rel="help"}` | link with attributes |
| `[[styled span]]{.pill bg="#eef"}` | inline span with attributes |
| `{{reviewer}}` | variable lookup (alias of `{{vars.reviewer}}`) |
| `{{meta.title}}` | namespaced variable |
| `[^fn-1]` | footnote reference |
| `[@smith2024]` | citation reference |
| `@[heading-id]` | cross-reference to a block id |
| `@{user:alice}` | typed mention |

Escape any of these with a backslash. `\*` stays a literal asterisk.

## Block syntax

NODX has one block form. A *delimited block* opens with one or more colons,
a name, optional attributes, and closes with the same number of colons:

```nodx
::note {type="warning"}
Lite form (two colons). Easier to type.
::

:::section
Full form (three or more colons). Required when you need to nest blocks of
the same colon count.
:::
```

### Useful built-in blocks

| Block | Purpose |
|---|---|
| `note` | callout / aside (renders as `<aside>`) |
| `section` | logical grouping (renders as `<section>`) |
| `figure` + `caption` | image / diagram with caption |
| `image` | bare image (block form) |
| `table` (or pipe tables) | tabular data |
| `code`, `pre` | literal block, never re-parsed |
| `math` | block math, raw |
| `style` | inline NODS stylesheet (safe subset only) |
| `toc` | generated table of contents |
| `pagebreak` | explicit page break for print/export |
| `quote` | block quotation |
| `bibliography` | reference list |
| `speaker-notes` | non-displayed presenter notes |
| custom `::your-component` | rendered with a registered template, or falls back to a safe `<section>` |

### Pipe tables

```nodx
| Metric  | Value |
|---------|------:|
| Revenue |  120K |
| Costs   |   80K |
```

Right- or left-align columns by putting `:` on the dash row.

### Lists

```nodx
- Unordered item
- Another item

1. Ordered item
2. Another one

- [ ] Open task
- [x] Done task
```

## Attributes

Every block and most inlines accept an attribute block in `{…}`:

```nodx
{#section-id .class-a .class-b color="red" data-thing="value"}
```

- `#name` becomes the id (must match `[A-Za-z][A-Za-z0-9-]*`).
- `.name` adds a class.
- `key="value"` becomes a typed attribute, with value quoting if it contains
  whitespace.
- A handful of *safe shorthands* are translated into inline CSS — see the
  [Theming reference](../reference/themes.md).

Inline spans accept the same syntax plus a *class suffix* form:

```nodx
[[Approved]].status.success
```

## Variables and front matter

```nodx
---
title: Q1 report
vars:
  reviewer: Alice
  threshold: 95
---

Reviewer: {{reviewer}}. Target threshold: {{threshold}}%.
```

`{{name}}` is shorthand for `{{vars.name}}`. `{{meta.title}}` reaches any
key under front matter `meta:`.

Front matter is the *only* place where information can affect parsing
(profiles, language) or rendering (theme, components). Body content cannot
re-enter the metadata.

## Profiles

A document declares which profiles it depends on:

```nodx
---
profiles:
  requires:
    - core
    - rich
  optional:
    - style
    - agent-read
---
```

Built-in profiles in 1.0:

| Profile | Adds |
|---|---|
| `plain` | bare paragraphs, headings, inline emphasis only |
| `core` | everything in `plain` plus blocks, links, lists, tables |
| `rich` | figures, footnotes, citations, forms, mentions |
| `style` | safe NODS stylesheets via `::style` and front-matter `theme:` |
| `package` | the document expects a `.nodx` ZIP container |
| `agent-read` | NCP projection is part of the contract |

A reader that does not support a *required* profile fails with `NODX-E024`
and exit code `3`. Optional profiles are best-effort.

## What NODX intentionally does not have

- No scripts. No macros. No active content.
- No remote includes by default. The host application owns network I/O, not
  the parser.
- No "raw HTML inside the document" escape hatch. If a feature is not in the
  AST, it does not exist for downstream tools.
- No fluffy syntax that produces different ASTs in different parsers. Two
  conformant implementations always emit the same canonical JSON.

That is the whole tour. The rest of this site is detail.

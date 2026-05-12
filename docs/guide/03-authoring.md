# Authoring guide

How to write NODX documents that are pleasant to read, easy to review, and
robust under tools you do not control yet.

## Pick the right shape

Three shapes cover most documents. Pick the smallest one that fits.

### Minimal — a memo, a note, a README

```nodx
---
title: Onboarding checklist
---

# Onboarding checklist #intro

Welcome.

- [ ] Pick a laptop
- [ ] Set up your accounts
- [ ] Join the standup
```

No profile declaration: the parser supplies `core` by default. No theme: the
renderer chooses a sensible one.

### Intermediate — a report, a spec, a piece of documentation

```nodx
---
title: Q1 report
theme: web
language: en
---

::toc {title="Contents" depth="2"}
::

# Quarterly report #q1

## Headline metrics #q1-metrics

| Metric  | Q4 2025 | Q1 2026 | Delta |
|---------|--------:|--------:|------:|
| Revenue |     90K |    120K |  +33% |
| Costs   |     70K |     80K |  +14% |

## Highlights #q1-highlights

::note {type="info"}
We hit the revenue target two weeks early.
::
```

The TOC writes itself from the heading graph, the theme controls layout, and
the document still reads cleanly as plain text.

### Advanced — a packaged document with components and a theme

```nodx
---
schema: nodx/1.0
type: document
title: Internal release
language: en
profiles:
  requires:
    - core
    - rich
  optional:
    - style
    - package
components:
  - name: approval-card
    template: |
      :::note {class="nodx-component nodx-component--approval-card"}
      ## {{title}}
      {{children}}
      :::
---

::approval-card {title="Approved"}
This release shipped on **2026-05-11**.
::
```

Use this form when the document needs an explicit contract — for CI, for
agents, or for archival.

## Use stable ids

Heading auto-slugs are convenient but fragile: renaming a section silently
breaks every link to it. Prefer explicit ids:

```nodx
# Q1 results #q1
## Highlights #q1-highlights
```

A few rules:

- An id is `[A-Za-z][A-Za-z0-9-]*` (no underscores, no leading digit).
- Ids must be unique within a document; duplicates raise `NODX-E006`.
- An id can be 1–256 bytes long.

The validator catches mismatches before they become broken anchors.

## Prefer named blocks over class-driven HTML

Don't:

```nodx
:::section {.warning-box}
Read this carefully.
:::
```

Do:

```nodx
::note {type="warning"}
Read this carefully.
::
```

Reason: named blocks have semantics in the AST. Tools (TOC, NCP, exporters)
understand `note` but cannot infer meaning from a class name. The HTML
renderer still gives you a styleable `<aside>` either way.

## Lean on profiles

Profiles tell readers what your document needs. Two reasons to declare them:

1. **Fail fast in CI.** If a reader does not implement `rich`, the document
   is rejected with exit code `3` instead of silently dropping figures and
   citations.
2. **Document intent.** Reviewers can tell at a glance that you rely on
   forms (`rich`), custom styles (`style`), or agent extraction
   (`agent-read`).

If you do not declare `profiles`, the parser injects `requires: [core]`.

## Keep style blocks small

Anything you can express via theme tokens, *do* express via tokens:

```nodx
::style
:root {
  --nodx-color-primary: #1d4ed8;
  --nodx-font-heading: Georgia, serif;
}
::
```

This survives theme switches and exports. Heavy per-element CSS in a `::style`
block survives less well — sanitization is strict, exports may ignore it,
and tools like the NCP projection see only the AST anyway. See the
[Theming reference](../reference/themes.md) for the full token list.

## Embed assets in a package, not in the source

For documents that need images, fonts, or component templates, ship them in
a `.nodx` ZIP container:

```
my-doc.nodx        ← packaged form, .zip file
├─ mimetype
├─ manifest.yaml
├─ doc.nodx        ← actual NODX source
├─ assets/
│  ├─ cover.png
│  └─ logo.svg
└─ themes/
   └─ brand.nods
```

You build it with `scripts/build_package.py` and inspect it with
`nodx package inspect path/to/doc.nodx`. Packaged documents are immutable
units: every entry has its size and SHA-256 in the manifest, the reader
verifies both before exposing the bytes, and nothing is ever extracted to
disk.

## Variables for "this changes per render"

```nodx
---
vars:
  reviewer: Alice
  release: 1.0.3
---

Released by {{reviewer}} as **v{{release}}**.
```

Use variables for the small set of values that should change per render
without touching the body. Don't reach for them when a normal sentence will
do — a document with thirty unfilled variables is a template, not a document.

## Plan for long documents

The reference implementation parses a 2000-page synthetic book (~50 nodes
per page, ~3.8 MB source, ~108k AST nodes) in well under a second, peaking
around 80 MB of RAM. There are real limits, though, and you should know
about them:

| Cap | Default | What happens when you hit it |
|---|---|---|
| `source_bytes` | 64 MiB | parser aborts with `NODX-E012` |
| `line_length` | 1 MiB | parser warns with `NODX-E012` and continues |
| `nodes_per_document` | 100 000 | parser stops emitting nodes, raises `NODX-E012` |
| `block_nesting_depth` | 32 | parser stops descending, raises `NODX-E012` |

For documents that legitimately need more — say, a 5000-page reference
manual — pass higher limits through `parse_str_with_limits`, or set them via
the library API of your integration. The [Internals: limits and
scalability](../internals/scalability.md) page has the full picture.

## Review NODX documents the right way

The AST is canonical, so `git diff` on the `.nodx` source is meaningful. For
deeper review, compare the canonical AST:

```sh
nodx ast doc.nodx | jq . > doc.ast.json
```

Two semantically identical documents always produce the same canonical AST
even if their source whitespace differs. That makes "did this rewording
change meaning?" a one-line answer.

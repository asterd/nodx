# NODX Documentation

This directory holds every document about NODX that is not the normative
specification itself. The structure mirrors how people actually use the
project:

- **[guide/](./guide/)** — Tutorials and walkthroughs. Start here if you
  are writing your first document or wiring NODX into a tool.
- **[reference/](./reference/)** — Exhaustive descriptions of every
  construct, code, profile, theme, and resource limit. The kind of page
  you keep open while implementing or reviewing.
- **[cookbook/](./cookbook/)** — Recipes for specific tasks: CI
  integrations, search indexes, packaging, signing, retrieval pipelines.
- **[internals/](./internals/)** — Architecture, scalability numbers,
  security model, evolution notes. For contributors and integrators
  who want to understand what is and is not load-bearing.

The normative specification lives one level up:

- **[NODX-RFC-0001.md](../NODX-RFC-0001.md)** — the 1.0 contract.
- **[SECURITY.md](../SECURITY.md)** — security policy and reporting.

For implementers building their own NODX parser, the right reading
order is:

1. [NODX-RFC-0001](../NODX-RFC-0001.md) — the contract you have to meet.
2. [reference/conformance.md](./reference/conformance.md) — how
   compliance is measured.
3. [reference/ast.md](./reference/ast.md) — what the parser must emit.
4. [reference/diagnostics.md](./reference/diagnostics.md) — the lint
   surface every implementation shares.
5. [internals/architecture.md](./internals/architecture.md) — how the
   reference implementation is laid out, in case you want a head start.

## Guide pages

| | |
|---|---|
| [01 — Quickstart](./guide/01-quickstart.md) | Install, write, render in five minutes. |
| [02 — Syntax tour](./guide/02-syntax-tour.md) | Every construct, with side-by-side examples. |
| [03 — Authoring guide](./guide/03-authoring.md) | Patterns for real documents. |
| [04 — Packaging](./guide/04-packaging.md) | The `.nodx` ZIP container, manifests, signatures. |
| [05 — CLI](./guide/05-cli.md) | All subcommands, exit codes, common pipelines. |

## Reference pages

| | |
|---|---|
| [AST](./reference/ast.md) | Canonical document, node, and inline shapes. |
| [Blocks](./reference/blocks.md) | Every built-in block, its HTML output, its attributes. |
| [Inline](./reference/inline.md) | Inline tokens, emphasis, links, variables, references. |
| [Attributes](./reference/attributes.md) | The attribute block grammar and reserved keys. |
| [Diagnostics](./reference/diagnostics.md) | The `NODX-Exxx` catalog. |
| [Profiles](./reference/profiles.md) | Profile names, what they require, how to declare them. |
| [Themes and styling](./reference/themes.md) | Built-in themes, design tokens, safe NODS subset. |
| [Limits](./reference/limits.md) | Default resource caps and how to tune them. |
| [NCP](./reference/ncp.md) | Agent-readable content projection. |
| [Conformance](./reference/conformance.md) | The 1.0 conformance package and how to run it. |

## Cookbook recipes

The [cookbook](./cookbook/) has standalone recipes for: rendering in CI,
embedding HTML, semantic diffs, hashing for cache invalidation, building
search indexes, signing release artifacts, exporting to PDF/DOCX/PPTX,
bundling assets, and feeding documents to agents.

## Internals

| | |
|---|---|
| [Architecture](./internals/architecture.md) | Crate graph, per-crate responsibilities. |
| [Scalability](./internals/scalability.md) | Measured numbers for 2 000- to 10 000-page documents. |
| [Streaming evolution](./internals/streaming-evolution.md) | Design notes for a future streaming pipeline. |
| [Security model](./internals/security-model.md) | Threat model and load-bearing safety properties. |

## Other artifacts

- **[ECOSYSTEM_PLAN.md](./ECOSYSTEM_PLAN.md)** — adoption roadmap.
- **[IMPLEMENTER_GUIDE.md](./IMPLEMENTER_GUIDE.md)** — pointer to the
  reference pages above, kept for backward compatibility with older
  links.
- **[THEMING.md](./THEMING.md)** — the original theming notes; the
  authoritative version is now [reference/themes.md](./reference/themes.md).
- **[RFC-0003 — Paged output contract](./RFC-0003-paged-output-contract.md)** —
  draft, post-1.0.
- **[RFC-0004 — Editor contract](./RFC-0004-editor-contract.md)** — draft,
  post-1.0.
- **[RFC-0005 — Parquet/Arrow portability](./RFC-0005-parquet-arrow-portability.md)** —
  draft, post-1.0.

Historical planning, migration, conformance snapshots, and wave prompts
are archived under [`archive/`](./archive/).

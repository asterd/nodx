# NODX

NODX is a UTF-8, text-first document format for structured, portable source
documents. A `.nodx` file can be plain text or a packaged ZIP; readers detect
the representation from the first bytes.

The normative specification is [NODX-RFC-0001](./NODX-RFC-0001.md).

## What This Repo Contains

- Rust reference implementation: parser, validator, URL policy, style safety,
  HTML renderer, TUI renderer, package reader, NCP projection, signing,
  editor/CST support, agent mutation SDK, and export previews.
- Independent JavaScript parser/projection in `packages/nodx-js`.
- Conformance, negative, rendering, security, package, export, and presentation
  fixtures under `spec/tests`.
- Implementer-facing conformance package under `spec/conformance/v1.0`.
- Showcase documents under `examples`.
- Editor starter integrations under `editors`.

## Example

```nodx
---
schema: nodx/1.0
type: document
title: Hello NODX
profiles:
  requires:
    - core
    - rich
  optional:
    - style
    - agent-read
---

:::toc {#contents role="primary" depth="2" title="Contents"}
:::

# Hello NODX {#hello}

This is **structured text** with a safe [link](https://example.com).

| Feature | Status |
| - | - |
| Canonical AST | stable |
| NCP semantic projection | stable |

:::note {#safe-note type="info"}
Unknown renderers keep fallback children as ordinary document content.
:::
```

## Build And Verify

Use `rtk` for repository commands:

```sh
rtk cargo test
rtk node --test packages/nodx-js/test/*.mjs
rtk sh scripts/run_conformance.sh
rtk sh scripts/verify_conformance_package.sh
rtk git diff --check
```

The conformance runner compares Rust and JavaScript canonical AST, NCP, and
diagnostics output for the committed fixture corpus and writes
`target/conformance-report.json`.

## CLI

```sh
rtk cargo build -p nodx
target/debug/nodx ast examples/showcase-web.nodx
target/debug/nodx validate examples/showcase-web.nodx --format json
target/debug/nodx html examples/showcase-web.nodx > target/showcase.html
target/debug/nodx tui examples/showcase-tui.nodx
target/debug/nodx ncp examples/showcase-web.nodx
target/debug/nodx package inspect examples/extended-showcase-bundled.nodx
target/debug/nodx package verify examples/extended-showcase-bundled.nodx
```

Exit codes follow the RFC: `0` success, `1` I/O or CLI usage failure, `2`
parse/validation/security failure, `3` unsupported required capability.

## Run The Examples

Build the CLI once:

```sh
rtk cargo build -p nodx
```

Validate a single example before rendering it:

```sh
target/debug/nodx validate examples/showcase-web.nodx --format json
```

Start the interactive web app from the repository root:

```sh
python3 -m http.server 8080
```

Then open:

```text
http://127.0.0.1:8080/apps/web/
```

The web app lets you select committed examples, including the packaged ZIP
showcase, edit the NODX source on the left, inspect the rendered document,
Canonical AST, and diagnostics on the right, toggle page simulation for
`:::pagebreak`, follow clickable TOC anchors, and export the current HTML
preview. The playground variant is available at:

```text
http://127.0.0.1:8080/apps/web/playground.html
```

Generate one static HTML file when you want a renderer artifact without the
interactive app:

```sh
mkdir -p target/examples
cp -R examples/assets target/examples/assets
target/debug/nodx html examples/showcase-web.nodx > target/examples/showcase-web.html
```

Run the terminal showcase:

```sh
target/debug/nodx tui examples/showcase-tui.nodx
```

Run the local desktop-style viewer for a single file:

```sh
python3 apps/desktop/nodx_viewer.py examples/showcase-web.nodx
```

Generate the agent-readable NCP projection:

```sh
target/debug/nodx ncp examples/showcase-web.nodx > target/showcase-web.ncp.json
```

Inspect and verify the packaged ZIP example:

```sh
target/debug/nodx package inspect examples/extended-showcase-bundled.nodx
target/debug/nodx package verify examples/extended-showcase-bundled.nodx
```

Render print and internationalization examples through the same HTML path:

```sh
target/debug/nodx html examples/print/print-portrait.nodx > target/examples/print-portrait.html
target/debug/nodx html examples/print/print-landscape.nodx > target/examples/print-landscape.html
target/debug/nodx html examples/i18n/arabic-rtl.nodx > target/examples/arabic-rtl.html
target/debug/nodx html examples/i18n/chinese-cjk.nodx > target/examples/chinese-cjk.html
target/debug/nodx html examples/i18n/mixed-scripts.nodx > target/examples/mixed-scripts.html
```

Run preview exports with machine-readable loss reports:

```sh
target/debug/nodx export pdf examples/showcase-web.nodx -o target/showcase.pdf
target/debug/nodx export docx examples/showcase-web.nodx -o target/showcase.docx
target/debug/nodx export pptx spec/tests/presentation/presentation-basic.nodx -o target/presentation.pptx
```

Run the full committed example/conformance sweep:

```sh
rtk sh scripts/run_conformance.sh
```

## Showcase

- `examples/showcase-web.nodx`: primary HTML showcase for the implemented 1.0
  web-safe surface: front matter, profiles, TOC, headings, inline syntax,
  tables, figures/images, safe style blocks, custom components with fallback,
  footnotes/citations, forms, and NCP-friendly IDs.
- `examples/showcase-tui.nodx`: terminal/desktop-oriented showcase for content
  where text, navigation, notes, code, page breaks, speaker notes, and
  fallback rendering matter more than CSS.
- `examples/extended-showcase-bundled.nodx`: packaged ZIP sample generated by
  `scripts/build_package.py`.
- `examples/layout-fonts.nodx`: visual showcase for horizontal layout, grid,
  asymmetric margins/padding, and different font-family rows.

The older focused examples remain useful as small regression fixtures.

## Security

NODX processors are expected to fail closed for untrusted input: no script
execution, no default network fetches, no package extraction to disk, strict
resource limits, safe URL/path policy, and context escaping for renderers.

See [SECURITY.md](./SECURITY.md) for the security policy.

## Documentation Layout

Root documentation is intentionally small:

- [NODX-RFC-0001.md](./NODX-RFC-0001.md): final 1.0 specification.
- [README.md](./README.md): project entry point.
- [SECURITY.md](./SECURITY.md): security policy.

Historical planning, migration, conformance snapshot, threat-model, and wave
prompt documents are archived under `docs/archive/` to keep the project root
readable without losing context.

For external implementers, start with [docs/IMPLEMENTER_GUIDE.md](./docs/IMPLEMENTER_GUIDE.md)
and [spec/conformance/v1.0](./spec/conformance/v1.0/README.md). The ecosystem
roadmap is [docs/ECOSYSTEM_PLAN.md](./docs/ECOSYSTEM_PLAN.md).

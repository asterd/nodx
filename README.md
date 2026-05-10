# NODX 0.1 Reference Implementation

Minimal public implementation for the NODX 0.1 draft in this directory.

Read the proposed RFC / working draft: [NODX 0.1 Working Draft](./NODX_0.1_Working_Draft.md).

NODX uses `.nodx` as a hybrid extension: a file can be UTF-8 Text NODX or a ZIP Packaged NODX, and readers identify the representation from the first bytes.

## What Is Included

- Rust reference implementation: `crates/nodx-core`
- CLI renderer/converter: `crates/nodx-cli`
- Independent JavaScript parser: `packages/nodx-js`
- Public conformance fixtures: `spec/tests/conformance`
- Example documents, including long-form, extended Rich/Style/Media/Component samples, three i18n examples (Arabic/RTL, Chinese/CJK, mixed Unicode scripts), pagination & print, and end-to-end typography: `examples`
- Desktop-style local viewer: `apps/desktop/nodx_viewer.py`
- Packaged NODX builder: `scripts/build_package.py`

The implemented surface covers Plain/Core plus a practical Rich subset (headings, paragraphs, delimited blocks, literal blocks including `:::style`, compact lists, pipe tables, canonical tables, attributes, common inline nodes, lang/dir propagation, focused semantic validation, safe HTML, TUI output, and a semantic NCP projection) and demonstrates Style Profile features through inline `:::style` blocks with sanitized CSS embedding.

## Verify

```sh
cargo test
sh scripts/run_conformance.sh
```

The conformance script compares canonical AST output from the Rust parser and the independent JavaScript parser for every fixture and example.

## CLI

```sh
cargo build -p nodx
target/debug/nodx ast examples/agent-workflow.nodx
target/debug/nodx html examples/rich-demo.nodx > /tmp/rich-demo.html
target/debug/nodx tui examples/complex-long-form.nodx
target/debug/nodx validate examples/extended-showcase.nodx
target/debug/nodx html examples/extended-showcase.nodx > /tmp/extended-showcase.html
target/debug/nodx html examples/typography.nodx > /tmp/typography.html
target/debug/nodx html examples/pagination.nodx > /tmp/pagination.html
target/debug/nodx html examples/i18n/arabic-rtl.nodx > /tmp/arabic.html
target/debug/nodx tui examples/i18n/chinese-cjk.nodx
target/debug/nodx ncp examples/agent-workflow.nodx
target/debug/nodx inspect examples/extended-showcase-bundled.nodx
target/debug/nodx ast examples/extended-showcase-bundled.nodx
```

## Package

Build the bundled `.nodx` ZIP package example:

```sh
python3 scripts/build_package.py
```

## Media Types

The draft proposes `text/nodx; charset=utf-8` for Text NODX and `application/nodx+zip` for Packaged NODX. Until registration, integrations should use documented experimental names such as `text/x-nodx` and `application/x-nodx+zip`.

## Desktop

```sh
python3 apps/desktop/nodx_viewer.py examples/agent-workflow.nodx
```

The desktop viewer uses only the Python standard library, the Rust CLI, and your default browser. It does not require Tkinter.

For non-interactive verification:

```sh
python3 apps/desktop/nodx_viewer.py --check examples/complex-long-form.nodx
```

## Print to PDF

NODX targets paged media through NODS `@page` rules. The reference engine does
not ship a native PDF renderer (deferred — see Intentional Gaps), but every
sample under `examples/print/` is print-ready: open the rendered HTML in a
browser and use **Cmd+P → Save as PDF** to produce a deterministic PDF file.

```sh
# A4 portrait report
target/debug/nodx html examples/print/print-portrait.nodx > /tmp/report.html
open /tmp/report.html  # then Cmd+P → Save as PDF

# A3 landscape dashboard
target/debug/nodx html examples/print/print-landscape.nodx > /tmp/dashboard.html
open /tmp/dashboard.html  # browser auto-detects landscape from @page size
```

Headless workflow with Chromium-based browsers:

```sh
# Generate PDF without opening a window
target/debug/nodx html examples/print/print-portrait.nodx > /tmp/report.html
chromium --headless --disable-gpu --print-to-pdf=/tmp/report.pdf /tmp/report.html
```

Both samples declare deterministic page geometry (`@page :first`,
`@page :left`, `@page :right`) and use only NODS constructs allowed by §17 of
the Working Draft. Forbidden CSS (animations, hover states, fixed positioning,
transforms, `attr()`, etc.) is detected by the parser and stripped from the
rendered output, with a `NODX-E027` warning in the diagnostics stream.

## Intentional Gaps

The full NODS cascade, signatures, lossless CST, complete URL resolver, native PDF/DOCX/PPTX exporters, and a complete YAML 1.2 safe-subset validator are not implemented in this minimal pass. The package reader handles only stored entries (no `deflate`). These are security-sensitive and should be added as separately tested modules.

The validator currently covers schema, required feature support, duplicate IDs, references, variables, safe asset paths, image alt text, simple table shape, custom-component fallback hints, direction attributes, and heading-level jumps. It is intentionally not a full Rich/Profile validator yet.

Inline `:::style` blocks are processed by a textual lexer that detects forbidden NODS constructs (animations, interactive selectors, layout escapes, `attr()`, `expression()`, `</style>` breakout, etc.) and:

1. emits `NODX-E027` warnings in `diagnostics`;
2. strips offending lines from the rendered HTML, replacing each with a `/* nodx-E027: forbidden NODS rule omitted */` marker.

A full CSS parser that validates the entire NODS allowlist (selectors, properties, at-rules, value functions, units) is a natural next step. The current lexer covers the patterns that break print-to-PDF determinism and the patterns explicitly listed in §17 of the Working Draft.

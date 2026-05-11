# NODX Reference Implementation

This repository contains the current public NODX reference implementation. The
code is still based on the `nodx/0.1` implementation surface while the 1.0
contract is being defined.

Active documents:

- [NODX 1.0 Working Draft](./NODX_1.0_Working_Draft.md): target 1.0 contract.
- [NODX 1.0 Evolution Plan](./NODX_1.0_Evolution_Plan.md): implementation roadmap.
- [NODX 0.1 Working Draft](./NODX_0.1_Working_Draft.md): historical input.
- [Implementation Plan](./IMPLEMENTATION_PLAN.md): current implementation status.

NODX uses `.nodx` as a hybrid extension. A file can be UTF-8 Text NODX or a ZIP
Packaged NODX, and readers identify the representation from the first bytes.

## Current Implementation

Implemented today:

- Rust reference crate: `crates/nodx-core`
- CLI facade: `crates/nodx-cli`
- Independent JavaScript parser: `packages/nodx-js`
- Public conformance fixtures: `spec/tests/conformance`
- Example documents under `examples`
- Desktop local viewer: `apps/desktop/nodx_viewer.py`
- Deterministic stored-ZIP package builder: `scripts/build_package.py`

The implemented behavior covers UTF-8 parsing, Plain/Core syntax, a practical
Rich subset, front matter, delimited blocks, headings, paragraphs, lists, pipe
tables, literal blocks, attributes, common inline nodes, deterministic
canonical JSON, focused semantic validation, safe HTML rendering, TUI rendering,
semantic NCP projection, and a minimal stored-ZIP package reader with manifest
digest verification.

The workspace does not yet contain separate `nodx-validate`, `nodx-url`,
`nodx-package`, `nodx-style`, `nodx-ncp`, or `nodx-render-html` crates. Those
are roadmap targets, not current modules.

## Verify

Use `rtk` when running repository commands:

```sh
rtk cargo test
rtk sh scripts/run_conformance.sh
rtk git diff --check
```

The conformance script compares canonical AST output from the Rust parser and
the independent JavaScript parser for every fixture and example.

## CLI

Current commands:

```sh
rtk cargo build -p nodx
target/debug/nodx ast examples/agent-workflow.nodx
target/debug/nodx html examples/rich-demo.nodx
target/debug/nodx tui examples/complex-long-form.nodx
target/debug/nodx validate examples/extended-showcase.nodx
target/debug/nodx diagnostics examples/extended-showcase.nodx
target/debug/nodx ncp examples/agent-workflow.nodx
target/debug/nodx inspect examples/extended-showcase-bundled.nodx
```

The current CLI does not yet implement the full 1.0 CLI contract from
`NODX_1.0_Working_Draft.md`, including `--format` flags, profile flags, stable
diagnostic JSON, package subcommands, or exit code `3` for unsupported required
profiles.

## Package

Build the bundled `.nodx` ZIP package example:

```sh
rtk python3 scripts/build_package.py
```

The package reader currently handles stored ZIP entries and manifest digest
verification. Deflated entries, virtual filesystem work, signatures, and
advanced package policy are deferred.

## Media Types

The drafts propose `text/nodx; charset=utf-8` for Text NODX and
`application/nodx+zip` for Packaged NODX. Until registration, integrations
should use documented experimental names such as `text/x-nodx` and
`application/x-nodx+zip`.

## Desktop Viewer

```sh
rtk python3 apps/desktop/nodx_viewer.py examples/agent-workflow.nodx
```

For non-interactive verification:

```sh
rtk python3 apps/desktop/nodx_viewer.py --check examples/complex-long-form.nodx
```

The viewer uses only the Python standard library, the Rust CLI, and the default
browser. It does not require Tkinter.

## Intentional Gaps

The full NODS cascade, signatures, lossless CST, complete URL resolver, native
PDF/DOCX/PPTX exporters, stable 1.0 profile enforcement, and complete YAML 1.2
safe-subset validation are not implemented yet. These are security-sensitive
surfaces and should be added as separately tested milestones.

Current inline `:::style` blocks are processed by a textual safety lexer that
detects forbidden NODS constructs, emits `NODX-E027`, and strips offending CSS
from rendered HTML. A full NODS parser and allowlist validator remains future
work.

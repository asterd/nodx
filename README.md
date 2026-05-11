# NODX Reference Implementation

This repository contains the current public NODX reference implementation. The
code is still based on the `nodx/0.1` implementation surface while the 1.0
contract is being defined.

Active documents:

- [NODX 1.0 Working Draft](./NODX_1.0_Working_Draft.md): target 1.0 contract.
- [NODX 1.0 Evolution Plan](./NODX_1.0_Evolution_Plan.md): implementation roadmap.
- [NODX 0.1 Working Draft](./NODX_0.1_Working_Draft.md): historical input.
- [Implementation Plan](./IMPLEMENTATION_PLAN.md): current implementation status.
- [Conformance Report](./CONFORMANCE.md): release-gate corpus and DoD matrix.
- [Interop Notes](./INTEROP.md): AST, NCP, package, profile, and media type notes.
- [Migration Guide](./MIGRATION-0.1-TO-1.0.md): changes from 0.1 to 1.0.
- [Release Notes](./RELEASE_NOTES-1.0.md): known limitations and verification.

NODX uses `.nodx` as a hybrid extension. A file can be UTF-8 Text NODX or a ZIP
Packaged NODX, and readers identify the representation from the first bytes.

## Current Implementation

Implemented today:

- Rust reference crate: `crates/nodx-core`
- Rust URL policy crate: `crates/nodx-url`
- Rust validator crate: `crates/nodx-validate`
- Rust style safety crate: `crates/nodx-style`
- Rust HTML renderer crate: `crates/nodx-render-html`
- Rust package reader crate: `crates/nodx-package`
- Rust signature verification crate: `crates/nodx-sign`
- CLI facade: `crates/nodx-cli`
- Independent JavaScript parser: `packages/nodx-js`
- Public conformance fixtures: `spec/tests/conformance`
- Example documents under `examples`
- Desktop local viewer: `apps/desktop/nodx_viewer.py`
- Deterministic stored-ZIP package builder: `scripts/build_package.py`

The implemented behavior covers UTF-8 parsing, Plain/Core syntax, a practical
Rich subset, front matter, delimited blocks, headings, paragraphs, lists, pipe
tables, literal blocks, attributes, common inline nodes, deterministic
canonical JSON, focused semantic validation in `nodx-validate`, safe HTML
rendering in `nodx-render-html`, safe NODS subset validation in `nodx-style`,
centralized URL/resource policy, TUI rendering, semantic NCP
projection, and a safe stored-ZIP package reader with manifest digest
verification and read-only virtual filesystem access. The `nodx-sign` crate
implements the NODX Signature 1.1 verification profile for canonical AST
digests and ES256 compact JWS signatures.

The workspace does not yet contain a separate `nodx-ncp` crate. Semantic NCP is
implemented in `nodx-core` and in the independent JavaScript package; a split
crate remains a roadmap target.

## Verify

Use `rtk` when running repository commands:

```sh
rtk cargo test
rtk sh scripts/run_conformance.sh
rtk git diff --check
```

The conformance script compares canonical AST output and semantic NCP output
from the Rust parser and the independent JavaScript implementation for every
text fixture and example. It writes `target/conformance-report.json`.

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

The current CLI implements `nodx validate --profile <profile>`,
`nodx validate --format json`, `nodx diagnostics --format json`, and exit code
`3` for unsupported required profiles. It does not yet implement the full 1.0
CLI contract from `NODX_1.0_Working_Draft.md`, including package subcommands.

## Package

Build the bundled `.nodx` ZIP package example:

```sh
rtk python3 scripts/build_package.py
```

The package reader currently handles stored ZIP entries, manifest size and
digest verification, CRC checks, ZIP path validation, and read-only virtual
filesystem access. Deflated entries, signatures, and advanced package policy
are deferred.

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

The full NODS cascade, signature trust store UX, lossless CST, complete URL
resolver, native PDF/DOCX/PPTX exporters, stable 1.0 profile enforcement, and
complete YAML 1.2 safe-subset validation are not implemented yet. These are
security-sensitive surfaces and should be added as separately tested
milestones.

Current inline `:::style` blocks are processed by the `nodx-style` allowlist
validator. Forbidden NODS constructs emit deterministic `NODX-E027`
diagnostics, and unsafe rules are omitted from rendered HTML. Full cascade and
computed style remain future work.

The full numeric corpus targets and release-candidate fuzz budget from
`NODX_1.0_Evolution_Plan.md` are documented release limitations until completed
on the release branch.

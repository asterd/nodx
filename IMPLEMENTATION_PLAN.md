# NODX 0.1 Minimal Reference Implementation Plan

## Assumptions

- The root documentation outside `old/` is the active source for this implementation.
- Version `nodx/0.1` is the target. The implementation demonstrates Plain/Core and a practical subset of Rich.
- The public interop requirement is satisfied by two independent parsers producing the same canonical Semantic AST for the conformance corpus.
- `.nodx` is the single canonical user-facing extension. Text NODX and Packaged NODX are detected by bytes.

## Architecture

- `crates/nodx-core`: Rust reference parser, canonical AST serializer, HTML renderer, TUI renderer, and NCP projection.
- `crates/nodx-cli`: command line entrypoint: `ast`, `html`, `tui`, `ncp`, `diagnostics`.
- `packages/nodx-js`: independent JavaScript parser and canonical serializer.
- `apps/desktop`: standard-library local browser viewer backed by the Rust CLI.
- `scripts/build_package.py`: deterministic builder for a Packaged `.nodx` example.
- `spec/tests/conformance`: public fixture corpus.
- `examples`: human-readable demo documents, including an agent/LLM workflow example.

## Scope

Implemented now:

- UTF-8 input path through Rust `parse_bytes`.
- Front matter safe-subset parser for simple mappings, nested maps, arrays, strings, booleans, numbers, null.
- Compact headings, paragraphs, compact lists, pipe tables, delimited blocks, literal `code`/`pre`/`math`.
- Attribute blocks with IDs, classes, and quoted named attributes.
- Inline text, strong, emphasis, code spans, links, spans, refs, variables, inline math.
- Deterministic canonical JSON with sorted object keys.
- Focused semantic validation for common Core/Rich correctness and safety issues.
- Safe HTML escaping and context-aware URL/asset checks.
- Recursive NCP semantic projection with deterministic SHA-256 hashes for LLM/agent context.
- Packaged `.nodx` sniffing and a minimal stored-ZIP package reader with manifest digest verification for generated examples.

Deferred deliberately:

- Compressed ZIP entries, signature verification, full NODS cascade, PDF/DOCX/PPTX exporters, lossless CST, full YAML 1.2 parser, complete URL policy resolver, full agent mutation SDK.
- These are larger security-sensitive surfaces and should be added only with focused tests and threat models.

## Verification

Run:

```sh
cargo test
sh scripts/run_conformance.sh
```

Manual rendering:

```sh
target/debug/nodx tui examples/agent-workflow.nodx
target/debug/nodx validate examples/extended-showcase.nodx
target/debug/nodx html examples/rich-demo.nodx > /tmp/rich-demo.html
target/debug/nodx ncp examples/agent-workflow.nodx
python3 apps/desktop/nodx_viewer.py examples/agent-workflow.nodx
```

## Performance Notes

- Parsing is line-oriented and single pass for block structure.
- Inline parsing is deterministic and avoids regex backtracking in Rust.
- Canonical serialization writes directly into a `String`.
- Renderers stream into output strings and escape per context.
- Current implementation favors small constant factors and no dependency startup cost.

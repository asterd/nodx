# NODX 1.0 Ecosystem Implementation Plan

Goal: make NODX 1.0 adoptable by external tools without requiring them to read
the Rust reference implementation.

## Phase 1: Stable Conformance Distribution

Deliverables:

- Publish `spec/conformance/v1.0` as a downloadable archive.
- Include fixture manifest, source documents, expected AST, expected NCP,
  expected diagnostics, and reference HTML.
- Add CI that runs `scripts/verify_conformance_package.sh`.

Acceptance:

- Any independent implementation can validate itself without building the full
  repository.

## Phase 2: Python Library

Package: `nodx`

Scope:

- `parse_bytes(bytes) -> Document`
- `canonical_json(document) -> str`
- `diagnostics(document) -> list[Diagnostic]`
- `ncp_json(document) -> str`
- package detection and safe package reader

Constraints:

- No network access.
- No unsafe YAML loader.
- No renderer-side execution.
- Match conformance package byte-for-byte for AST, NCP, and diagnostics.

Acceptance:

- Python test suite consumes `spec/conformance/v1.0/manifest.json`.
- Wheels publish for current CPython versions.

## Phase 3: JavaScript Package Hardening

Package: `@nodx/nodx`

Scope:

- Keep independent parser.
- Add package export map.
- Add TypeScript declarations.
- Add browser-safe parsing mode.
- Consume conformance package in npm tests.

Acceptance:

- `npm test` validates the conformance package and the repository corpus.

## Phase 4: VSCode Extension

Package: `nodx-language`

Scope:

- TextMate grammar and language configuration.
- Snippets for common blocks.
- Diagnostics via configured `nodx` CLI path.
- HTML preview command.
- Outline command from `nodx ncp`.

Acceptance:

- Opening `.nodx` highlights syntax.
- Saving a file updates diagnostics.
- Preview works without network access.

## Phase 5: Sublime Text And Notepad++

Scope:

- Ship syntax definitions.
- Document CLI validation workflow.
- Add snippets/templates where supported.

Acceptance:

- `.nodx` files open with syntax highlighting.
- Users can run the CLI externally for validation.

## Phase 6: Dedicated Viewer

Package: `nodx-viewer`

Scope:

- Local-only desktop viewer.
- Open text and packaged `.nodx`.
- Render HTML safely.
- Show diagnostics, NCP outline, and package manifest.
- No remote fetches by default.

Acceptance:

- Can inspect `examples/showcase-web.nodx`,
  `examples/showcase-tui.nodx`, and `examples/extended-showcase-bundled.nodx`.

## Phase 7: Registry And Interop

Deliverables:

- Media type registration draft for `text/nodx` and `application/nodx+zip`.
- File extension registration notes.
- Public compatibility matrix for Rust, JS, Python, VSCode, Sublime, Notepad++,
  and viewer.

Acceptance:

- Integrators have stable identifiers, examples, and conformance artifacts.

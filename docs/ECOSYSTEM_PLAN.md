# NODX 1.0 Ecosystem Implementation Plan

Goal: make NODX 1.0 adoptable by authors and external tools without requiring
them to read the Rust reference implementation.

Adoption has three audiences:

- Authors write `.nodx` like structured Markdown and should start from the
  README quickstart, examples, themes, and editor support.
- Integrators use the CLI, Canonical AST, NCP, packages, and exports in
  pipelines.
- Implementers build compatible parsers and validate against conformance
  fixtures.

## Phase 1: Stable Conformance Distribution

Deliverables:

- Publish `spec/conformance/v1.0` as a downloadable archive.
- Include Lite syntax fixtures: `::` blocks, heading light IDs, `{{name}}`,
  link attributes, Markdown-compatible table separators, and TOC overrides.
- Include fixture manifest, source documents, expected AST, expected NCP,
  expected diagnostics, and reference HTML.
- Add CI that runs `scripts/verify_conformance_package.sh`.
- Publish `spec/error-registry.json` for editors and CI tools.

Acceptance:

- Any independent implementation can validate itself without building the full
  repository.
- Rust and JavaScript pass the same fixture corpus byte-for-byte.

## Phase 2: Authoring Experience

Deliverables:

- Quickstart and cheat sheet aimed at Markdown users.
- Examples index that explains which file to open first.
- Standard themes documented as author choices: `none`, `base`, `web`,
  `print`, and `presentation`.
- Style guide for safe NODS/CSS and YAML style blocks.
- Accessibility guide that explains `alt`, headings, TOC labels, table scope,
  and link labels without assuming HTML expertise.

Acceptance:

- A new user can create and render a valid document in under 30 minutes.
- The README never requires Canonical AST knowledge for basic authoring.

## Phase 3: Python Library

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

## Phase 4: JavaScript Package Hardening

Package: `@nodx/nodx`

Scope:

- Keep independent parser.
- Add package export map.
- Add TypeScript declarations.
- Add browser-safe parsing mode.
- Consume conformance package in npm tests.

Acceptance:

- `npm test` validates the conformance package and the repository corpus.

## Phase 5: VSCode Extension

Package: `nodx-language`

Scope:

- TextMate grammar and language configuration.
- Snippets for common blocks.
- Diagnostics via configured `nodx` CLI path.
- HTML preview command.
- Outline command from `nodx ncp`.
- Quick fixes for common authoring issues: missing `alt`, duplicate IDs,
  heading jumps, and unsafe links.

Acceptance:

- Opening `.nodx` highlights syntax.
- Saving a file updates diagnostics.
- Preview works without network access.

## Phase 6: Sublime Text And Notepad++

Scope:

- Ship syntax definitions.
- Document CLI validation workflow.
- Add snippets/templates where supported.

Acceptance:

- `.nodx` files open with syntax highlighting.
- Users can run the CLI externally for validation.

## Phase 7: Dedicated Viewer

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

## Phase 8: Registry And Interop

Deliverables:

- Media type registration draft for `text/nodx` and `application/nodx+zip`.
- File extension registration notes.
- Public compatibility matrix for Rust, JS, Python, VSCode, Sublime, Notepad++,
  and viewer.

Acceptance:

- Integrators have stable identifiers, examples, and conformance artifacts.

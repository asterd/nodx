# NODX Implementation Plan

This file describes the current implementation honestly and points to
`NODX_1.0_Evolution_Plan.md` for the milestone roadmap. The active 1.0 contract
draft is `NODX_1.0_Working_Draft.md`. The 0.1 draft remains historical input.

## Current Status

The repository currently implements a minimal public `nodx/0.1` reference
engine and is preparing for NODX 1.0.

Current workspace:

```text
crates/
  nodx-core/
  nodx-package/
  nodx-url/
  nodx-validate/
  nodx-style/
  nodx-render-html/
  nodx-cli/
packages/
  nodx-js/
spec/tests/conformance/
examples/
scripts/
apps/desktop/
```

Current crates and packages:

- `crates/nodx-core`: Rust parser, canonical AST serializer, shared navigation
  graph resolver, TUI renderer, and NCP projection.
- `crates/nodx-package`: safe stored-ZIP package reader, manifest verifier, and
  read-only `PackageFs`.
- `crates/nodx-url`: centralized URL classification, package-relative path
  normalization, `ResourcePolicy`, and shared `ResourceLimits`.
- `crates/nodx-validate`: semantic validator, profile handling, and stable
  diagnostic JSON helpers.
- `crates/nodx-style`: safe NODS subset validator and sanitizer.
- `crates/nodx-render-html`: safe standalone HTML renderer.
- `crates/nodx-cli`: command line facade for `ast`, `html`, `tui`, `ncp`,
  `diagnostics`, `validate`, and `inspect`.
- `packages/nodx-js`: independent JavaScript parser, canonical serializer,
  semantic NCP projector, and shared diagnostics subset used for conformance
  parity.

The repository does not yet have a separate `nodx-ncp` crate, fuzz targets, or
complete golden corpus directories. Those are 1.0 roadmap work items.

## Implemented Behavior

Implemented now:

- UTF-8 input through Rust `parse_bytes`.
- Text NODX and stored-ZIP Packaged NODX sniffing.
- Safe front matter parser for simple mappings, nested maps, arrays, strings,
  booleans, numbers, null, and the NODX 1.0 forbidden YAML construct set.
- Compact headings, paragraphs, compact lists, pipe tables, delimited blocks,
  literal `code`, `pre`, `math`, and inline `style` blocks.
- Attribute blocks with IDs, classes, and quoted named attributes.
- Inline text, strong, emphasis, code spans, links, spans, refs, variables, and
  inline math.
- Deterministic canonical JSON with sorted object keys.
- Focused semantic validation for common Core/Rich correctness and safety
  issues.
- Safe HTML escaping, CSP emission, and centralized context-aware URL and asset
  checks.
- Safe NODS subset validation with deterministic `NODX-E027` diagnostics and
  renderer-side omission of unsafe style rules.
- Recursive Rust and JavaScript NCP semantic projection with deterministic
  SHA-256 hashes and resolved `toc` navigation entries.
- Safe stored-ZIP package reader with manifest size/digest verification, CRC
  checks, package path validation, ZIP bomb controls, and read-only virtual FS.

## Known Gaps Against 1.0

Not implemented yet:

- Full package-level 1.0 profile declaration enforcement.
- Separate NCP crate.
- Deflated ZIP entries and advanced package policy.
- Full NODS cascade and computed style.
- Lossless CST, source maps, signatures, mutation SDK, and native PDF/DOCX/PPTX
  exporters.
- Full security corpus and fuzz targets.

## Wave 00 Contract Cleanup

Wave 00 is documentation and contract cleanup only.

Completed by this wave:

1. Add `NODX_1.0_Working_Draft.md` as the target 1.0 contract.
2. Preserve `NODX_0.1_Working_Draft.md` as historical input.
3. Add the profile model, minimal viable reader, resource limits, CLI exit
   semantics, unsupported feature behavior, error registry, and navigable
   document model to the 1.0 draft.
4. Clarify that `toc` is declarative and that previous/next navigation is
   renderer output derived from the navigation graph, not a source node or
   `toc` attribute.
5. Update README and this implementation plan to avoid claiming modules that do
   not exist yet.
6. Add `SECURITY.md` and `THREAT_MODEL.md` skeletons.

No parser, renderer, crate, fixture, canonical JSON, NCP, diagnostic, or CLI
exit-code behavior is changed by Wave 00.

## Verification

Baseline verification:

```sh
rtk cargo test
rtk sh scripts/run_conformance.sh
rtk git diff --check
```

Targeted tests should be added with each future behavior change. Fixture and
golden rewrites are required only when an external behavior change intentionally
changes canonical AST output, NCP output, diagnostics, or CLI exit codes.

## Next Milestones

Follow `NODX_1.0_Evolution_Plan.md`:

1. M1: split `nodx-core` mechanically without behavior changes.
2. M2: validator, profiles, diagnostics, and CLI contract.
3. M3: URL policy and resource limits.
4. M4: package reader hardening.
5. M5: style safety.
6. M6: JavaScript parity and NCP.
7. M7: release readiness, fixtures, security corpus, and documentation.

## Wave 06 JavaScript Parity

Completed by this wave:

1. Split `packages/nodx-js` into the target `src/` modules while preserving the
   existing `parser.mjs` import facade.
2. Added JavaScript semantic NCP output with resolved `toc` navigation entries.
3. Added a JavaScript diagnostics subset for parser-owned and shared validator
   errors covered by the public negative corpus.
4. Expanded conformance, NCP, navigation, negative, and security fixtures.
5. Expanded `scripts/run_conformance.sh` to compare Rust/JS canonical AST and
   Rust/JS semantic NCP output and write `target/conformance-report.json`.

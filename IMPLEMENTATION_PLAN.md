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
  nodx-sign/
  nodx-agent-sdk/
  nodx-cst/
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
- `crates/nodx-sign`: NODX Signature 1.1 digest and ES256 compact JWS
  verification.
- `crates/nodx-agent-sdk`: NODX Agent Mutate 1.1 local validated mutation SDK,
  including atomic batches and JSONL change records.
- `crates/nodx-cst`: NODX Editor 1.2 byte-preserving CST state, AST path
  mapping, local patch primitives, and minimal agent attribute rewrites.
- `crates/nodx-cli`: command line facade for `ast`, `html`, `tui`, `ncp`,
  `diagnostics`, `validate`, and `inspect`.
- `packages/nodx-js`: independent JavaScript parser, canonical serializer,
  semantic NCP projector, and shared diagnostics subset used for conformance
  parity.

The repository does not yet have a separate `nodx-ncp` crate. It has Wave 07
fuzz target entry points under `fuzz/`, but the full release-candidate fuzz
budget and larger numeric corpus targets remain documented release limitations.

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
- Signature profile verification over frozen 1.0 canonical AST digests,
  including detached and packaged compact JWS with ES256.
- Local validated agent mutations over parsed AST documents, with target
  resolution by ID/path/hash, required `beforeHash`, per-operation validation,
  atomic rollback, and deterministic `nodx/change/1.1` records.
- Byte-identical CST parse/emit for editor workflows, recoverable syntax
  preservation, CST-to-AST path mapping, local source patches, and
  validation-backed minimal source rewrites for supported agent attribute
  mutations.

## Known Gaps Against 1.0

Not implemented yet:

- Full package-level 1.0 profile declaration enforcement.
- Separate NCP crate.
- Deflated ZIP entries and advanced package policy.
- Full NODS cascade and computed style.
- Signature trust store UX, LLM mutation integration, and native PDF/DOCX/PPTX
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

1. M10 and later: continue post-1.1 profile work as scoped by the evolution
   plan.

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

## Wave 07 Release Gate

Completed by this wave:

1. Marked `NODX_1.0_Working_Draft.md` as the frozen 1.0 release contract.
2. Published `CONFORMANCE.md`, `INTEROP.md`, `MIGRATION-0.1-TO-1.0.md`, and
   `RELEASE_NOTES-1.0.md`.
3. Expanded `SECURITY.md` and `THREAT_MODEL.md` from skeletons into release
   gate documents.
4. Added `fuzz/README.md` and fuzz target entry points for parser, front
   matter, block parsing, inline parsing, attributes, URL policy, package
   reading, NODS validation, NCP serialization, and navigation resolution.
5. Documented the media type registration plan in `INTEROP.md`.

No parser, renderer, canonical JSON, NCP, diagnostic, or CLI exit-code behavior
is changed by Wave 07.

## Wave 08 Signature Profile

Completed by this wave:

1. Added `crates/nodx-sign`.
2. Added canonical AST digest helpers using
   `sha256-BASE64URL_WITHOUT_PADDING`.
3. Added compact JWS verification for detached and attached digest payloads.
4. Added mandatory ES256 verification with RustCrypto `p256`.
5. Kept EdDSA deferred with a dependency threat note in
   `crates/nodx-sign/README.md`.
6. Added `TrustPolicy` so key/trust decisions are separate from
   cryptographic validity.
7. Added detached and packaged signature verification tests and tamper vectors.

No parser, renderer, canonical JSON, NCP, diagnostic, or CLI exit-code behavior
is changed by Wave 08.

## Wave 09 Agent Mutate Profile

Completed by this wave:

1. Added `crates/nodx-agent-sdk`.
2. Added validated operations for insert, replace, delete, add-attribute,
   set-attribute, remove-attribute, add-comment, approve, and reject.
3. Added target resolution by ID, path, and node hash.
4. Required `beforeHash` for every mutation operation and computed `afterHash`
   in each emitted change record.
5. Validated after every operation with `nodx-validate`.
6. Kept failed batches atomic by applying to a cloned document until the full
   batch succeeds.
7. Added deterministic `nodx/change/1.1` JSONL change records and fixture-backed
   tests.

No parser, renderer, canonical JSON, NCP, diagnostic, or CLI exit-code behavior
is changed by Wave 09.

## Wave 10 Editor CST Profile

Completed by this wave:

1. Added `crates/nodx-cst`.
2. Preserved source bytes, line endings, delimiters, whitespace, attribute
   order, and recoverable invalid syntax as editor CST state.
3. Added CST node byte spans and AST path mapping.
4. Added parse-to-emit byte-identical round-trip behavior.
5. Added local patch primitives for replace, insert, delete, and named
   attribute updates that preserve surrounding source bytes.
6. Added CST-backed integration for `nodx-agent-sdk` attribute operations when
   they can be represented as minimal source patches.

No canonical AST hash, NCP semantic output, diagnostic, or CLI exit-code
behavior is changed by Wave 10.

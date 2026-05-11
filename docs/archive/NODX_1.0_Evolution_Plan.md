# NODX 1.0 Evolution Plan

**Purpose.** This document is the agent-executable roadmap for taking the current
`nodx/0.1` reference implementation to a stable `nodx/1.0` format and a credible
reference implementation, then continuing into `1.1`, `1.2`, and later profiles.

**Audience.** Coding agents, human maintainers, security reviewers, parser
authors, renderer authors, package implementers, editor implementers, and
agent/RAG integrators.

**Core decision.** NODX 1.0 is a stable document format and safe reference
implementation, not the full future ecosystem. The 1.0 release must be small
enough to finish and strict enough to trust.

**North star.** NODX must be:

1. easy to implement partially;
2. hard to implement unsafely;
3. deterministic where hashes, signatures, interop, and agents depend on it;
4. explicit about unsupported features;
5. useful as plain text before advanced packaging, signing, or editing exists.

**Non-negotiable 1.0 rule.** A minimal conforming reader must not need package
support, signatures, lossless CST, full NODS cascade, PDF/DOCX/PPTX export, or
agent mutations.

---

## Table of Contents

1. Current State
2. Release Architecture
3. Versioning and Compatibility
4. Profile Model
5. Security Baseline
6. Resource Limits
7. Workspace Target
8. Milestone Roadmap
9. Detailed Milestone Specs
10. CLI Contract
11. JavaScript Implementation Contract
12. Testing and Release Gates
13. Error Registry
14. Agent Execution Rules
15. Documentation Deliverables
16. Final Definitions of Done
17. Appendix A: Current Gaps
18. Appendix B: Deferred Profiles
19. Appendix C: Agent Handoff Template

---

## 1. Current State

This repository currently implements a minimal public `nodx/0.1` reference
engine.

### 1.1 Implemented today

- Rust reference crate: `crates/nodx-core`.
- CLI facade: `crates/nodx-cli`.
- Independent JavaScript parser: `packages/nodx-js/parser.mjs`.
- Public conformance fixtures: `spec/tests/conformance`.
- Examples for plain, rich, typography, i18n, print, agent workflow, and package.
- Desktop local viewer: `apps/desktop/nodx_viewer.py`.
- Deterministic stored-ZIP package builder: `scripts/build_package.py`.

Implemented behavior includes:

- UTF-8 parsing path in Rust.
- Plain/Core syntax and a practical Rich subset.
- Front matter parser for simple mappings, nested maps, arrays, strings,
  booleans, numbers, and null.
- Delimited blocks, headings, paragraphs, lists, pipe tables, literal blocks.
- Attribute blocks, IDs, classes, named attributes.
- Common inline nodes.
- Deterministic canonical JSON with sorted object keys.
- Focused semantic validation inside `nodx-core`.
- Safe HTML renderer and TUI renderer.
- NCP semantic projection with deterministic SHA-256 hashes.
- Minimal stored-ZIP package reader with manifest digest verification.
- Rust/JS conformance on current fixtures and examples.

### 1.2 Known implementation shape

At the time of this plan:

- `crates/nodx-core/src/lib.rs` is a 3,134-line monolith.
- `packages/nodx-js/parser.mjs` is a 489-line single-file parser.
- The Rust workspace contains only `nodx-core` and `nodx-cli`.
- The CLI has only `ast`, `html`, `tui`, `ncp`, `diagnostics`, `validate`, and
  `inspect`.
- The conformance runner compares Rust and JS canonical AST output.
- There is no `fuzz/` directory.
- There are no security corpus directories.
- There is no lossless CST, JWS signature support, full NODS cascade, package
  virtual filesystem, or mutation SDK.

### 1.3 Verification baseline

These commands are the baseline before any milestone work:

```sh
rtk cargo test
rtk sh scripts/run_conformance.sh
```

Any milestone that changes behavior must keep these commands green or explicitly
update the affected fixtures and documentation in the same change.

---

## 2. Release Architecture

The project has three separate layers. Agents must not collapse them into one
mega-release.

### 2.1 Format layer

The format layer defines the syntax, AST shape, canonicalization rules, error
registry, security model, and compatibility promises.

For 1.0, the format layer includes:

- Text NODX.
- Plain, Core, and Rich profiles.
- Optional Style syntax with safe rejection of unsupported features.
- Optional Package syntax and manifest rules.
- Declarative navigation nodes for generated tables of contents and local
  section navigation.
- Canonical Semantic AST.
- NCP semantic projection.
- Error registry.
- Security baseline.
- Resource limits.

### 2.2 Reference implementation layer

The reference implementation proves the format can be implemented safely.

For 1.0, the reference implementation includes:

- Rust parser.
- JavaScript parser.
- Rust validator.
- Rust safe HTML renderer.
- Rust and JS canonical AST parity.
- Rust and JS NCP semantic parity.
- Rust package reader for safe packages.
- CLI with stable exit semantics.
- Public fixture corpora and conformance report.

### 2.3 Ecosystem layer

The ecosystem layer includes advanced profiles and integrations. These are not
required for `nodx/1.0` unless explicitly listed in Section 16.

Examples:

- Lossless CST and editor rewrites.
- JWS signature verification.
- Agent mutation SDK.
- NCP lossless and summary modes.
- Full NODS cascade and computed style.
- DOCX/PPTX exporters.
- Browser playground.
- Python bindings.

These belong to `1.1`, `1.2`, or later unless a maintainer explicitly moves a
specific item into the 1.0 scope.

---

## 3. Versioning and Compatibility

### 3.1 Version identifiers

- Source schema: `schema: nodx/1.0`.
- Canonical Semantic AST schema: derived from the source schema.
- NCP semantic projection schema: `"schema": "nodx-ncp/1.0"`.
- Package manifest version: `"manifest_version": "1.0"`.
- Future change records: `"schema": "nodx/change/1.1"` or later.

### 3.2 Frozen at 1.0

Changing any of these after 1.0 requires a major version bump:

1. Plain/Core/Rich surface syntax.
2. Canonical Semantic AST JSON shape.
3. Canonical JSON serialization rules.
4. Error code numbers and default severities.
5. Required package layout fields.
6. Resource limit defaults and enforcement points.
7. CLI exit code semantics.
8. Profile declaration field names and fail-closed behavior.

### 3.3 Not frozen at 1.0

These may evolve in `1.x`:

1. Renderer output bytes, except where a golden test explicitly freezes output.
2. NCP summary heuristics.
3. Advanced package writer compression choices.
4. Style cascade internals.
5. Exporter fidelity details.
6. CLI flags marked `--unstable-*`.

### 3.4 Migration rule

Every breaking change from `nodx/0.1` to `nodx/1.0` must appear in
`MIGRATION-0.1-TO-1.0.md` with:

- old input;
- new input;
- expected diagnostic if an old construct is rejected;
- suggested mechanical patch when possible.

---

## 4. Profile Model

Profiles are capability declarations. They are not release milestones.

### 4.1 Profile tiers

| Tier | Profile | 1.0 status | Scope |
|---|---|---:|---|
| Required format | `NODX-Plain-1.0` | blocking | UTF-8 text, paragraphs, headings, safe escaping, implicit metadata. |
| Required format | `NODX-Core-1.0` | blocking | front matter, blocks, attrs, lists, inline base, diagnostics. |
| Required format | `NODX-Rich-1.0` | blocking | tables, figures, images, math text, footnotes, bibliography, forms, declarative navigation/TOC nodes, media fallbacks where specified. |
| Supported subset | `NODX-Style-1.0` | partial blocking | safe NODS allowlist and rejection. Full cascade may land after 1.0. |
| Supported subset | `NODX-Package-1.0` | blocking for package reader only | ZIP package read, manifest, assets, digest verification, no extraction. |
| Projection | `NODX-Agent-Read-1.0` | blocking | stable IDs, hashes, NCP semantic, read-only agent consumption. |
| Future | `NODX-Agent-Mutate-1.1` | deferred | validated mutations, batches, change records. |
| Future | `NODX-Signature-1.1` | deferred | JWS verification, trust hooks, signature corpus. |
| Future | `NODX-Editor-1.2` | deferred | lossless CST, source maps, local rewrites. |
| Future | `NODX-Presentation-1.2` | deferred | deck/slide/speaker-notes semantics and PPTX export. |

### 4.2 Declaration format

A document may declare which profiles it requires and which it can use
optionally:

```yaml
schema: nodx/1.0
profiles:
  requires: [core, rich]
  optional: [style, package]
```

Rules:

1. `profiles.requires` and `profiles.optional` are arrays of short names.
2. Valid short names at 1.0 are `plain`, `core`, `rich`, `style`, `package`,
   and `agent-read`.
3. Short names reserved for later versions are `agent-mutate`, `signature`,
   `editor`, and `presentation`.
4. Unsupported required profiles emit `NODX-E024` with severity `error`.
5. Unsupported optional profiles emit `NODX-E023` with severity `warning`.
6. A CLI build that cannot satisfy a required profile exits with code `3`.
7. Omitted `profiles` means the validator infers required features from used
   syntax and applies the same rules.
8. Package manifests must declare a profile set that is a superset of the entry
   document's required profiles.

### 4.3 Navigable document model

NODX 1.0 must support documents that are natively ready for modern navigable
documentation experiences without baking a specific screen layout into the
format.

The format-level feature is a declarative `toc` block in the Rich profile. A
document may contain zero, one, or many `toc` nodes. Each node declares which
part of the document it describes; renderers decide whether that becomes a
left navigation rail, a right in-chapter outline, a dropdown with submenus, a
print table of contents, or a textual fallback.

Example:

```nodx
:::toc {#main-nav role="primary" source="document" depth="2" title="Contents"}
:::

::::section {#chapter-3}
## Chapter 3

:::toc {#chapter-3-nav role="local" scope="#chapter-3" depth="3" title="In this chapter"}
:::

...
:::: section
```

Required 1.0 rules:

1. `toc` is a Rich block node with empty source content.
2. Multiple `toc` nodes are allowed.
3. A `toc` node may use `role="primary"`, `role="local"`,
   `role="secondary"`, or `role="breadcrumb"`.
4. A `toc` node may use `source="document"` or `scope="#id"`. If neither is
   present, `source="document"` is implied.
5. `scope="#id"` resolves to the subtree rooted at the referenced node.
6. `depth` is a positive integer from `1` to `6` and limits included heading
   levels relative to the selected source.
7. `min-level` and `max-level`, if present, are absolute heading levels from
   `1` to `6`; invalid values or ranges produce `NODX-E004`.
8. `title` is a plain text accessible label, not rendered inline content. If
   omitted, renderers use a deterministic default label based on `role`.
9. Styling uses normal IDs, classes, attributes, and the Style profile. The
   source format must not require fixed left/right column semantics.
10. The resolved entries are derived from existing `heading` and `section`
    nodes with stable IDs. Generated entries are not duplicated into the
    canonical AST as child nodes.
11. Unresolved `scope="#id"` references produce `NODX-E007`.
12. Renderers that support navigation should resolve entries deterministically
    in document order. Renderers that do not support navigation must render a
    safe fallback or emit `NODX-E015`.
13. The NCP semantic projection must expose resolved navigation entries so
    agents can understand document structure without renderer-specific HTML.
14. Previous/next navigation is renderer output derived from the resolved
    navigation graph. It is not represented as a source node or `toc` attribute
    in NODX 1.0.

Implementation note: the existing `toc` placeholder should evolve into this
model instead of adding a second node such as `nav` or `summary`. This keeps the
surface small and preserves compatibility with current examples.

### 4.4 Minimal viable reader

A minimal conforming reader may implement only Plain or only Core if it:

1. validates UTF-8;
2. rejects BOM and U+0000 according to policy;
3. enforces source size and line length limits;
4. escapes rendered text by output context;
5. blocks unsafe URL schemes;
6. never executes document content;
7. never fetches remote resources by default;
8. renders fallback children for unknown blocks where possible;
9. reports unsupported required features with `NODX-E024`;
10. exits with code `3` for unsupported required features in the CLI.

---

## 5. Security Baseline

NODX processors handling untrusted input must fail closed by default.

### 5.1 Default forbidden surface

```text
embedded scripts:               forbidden
macros:                         forbidden
plugins:                        forbidden
remote resource loading:        forbidden
file:// references:             forbidden
javascript:/vbscript: refs:     forbidden
remote styles/imports/includes: forbidden
active SVG:                     forbidden
inline SVG:                     forbidden at 1.0
inline MathML:                  forbidden at 1.0
media autoplay:                 forbidden
package extraction to disk:     forbidden
unknown component execution:    forbidden
unknown component fallback:     allowed
agent mutations:                not part of 1.0
filesystem writes during read:  forbidden
network access during read:     forbidden
```

### 5.2 Required enforcement order

Reference implementation processors enforce these checks in order:

1. input byte size;
2. UTF-8 validation;
3. BOM and U+0000 handling;
4. line length and front matter size;
5. front matter safe subset;
6. block and inline parse limits;
7. package path validation when packaged;
8. package ZIP bomb controls when packaged;
9. package manifest digest verification when packaged;
10. centralized URL policy;
11. NODS allowlist or safe rejection;
12. semantic validation;
13. renderer escaping by output context.

### 5.3 Network and filesystem policy

- Reading Text NODX must not write files.
- Reading Packaged NODX must not extract files to disk.
- Package readers expose a virtual read-only filesystem.
- No reference crate may open a network socket.
- Host integrations may fetch resources only outside the reference crates and
  only through an explicit host policy.

### 5.4 URL policy

URL handling is centralized in `nodx-url` by 1.0. Ad-hoc string checks are not
allowed after that crate exists.

Allowed schemes by reference kind:

| Kind | Allowed by default |
|---|---|
| `link` | `https`, `http`, `mailto`, `tel`, package-relative |
| `asset` | `data:` if size-limited and safe MIME, package-relative |
| `style` | package-relative |
| `include` | package-relative |
| `font` | package-relative |
| `media-fallback` | package-relative |

Forbidden everywhere by default:

- `javascript:`;
- `vbscript:`;
- `file:`;
- `jar:`;
- `chrome:`;
- `about:`;
- percent-encoded scheme bypasses;
- control bytes;
- backslash path tricks;
- path traversal;
- `data:text/html`;
- `data:application/xhtml+xml`;
- remote `@import`;
- remote NODS `url(...)`.

### 5.5 YAML front matter safe subset

Allowed:

- mapping;
- sequence;
- plain scalar;
- double-quoted scalar;
- single-quoted scalar;
- block literal scalar;
- block folded scalar.

Forbidden, all fatal `NODX-E019`:

1. anchors;
2. aliases;
3. explicit tags;
4. merge keys;
5. duplicate mapping keys;
6. multiple YAML documents;
7. non-string mapping keys;
8. non-finite numbers;
9. binary/octal/hex integer specials outside the accepted core schema;
10. native timestamps.

### 5.6 Package safety

The package reader rejects:

1. absolute paths;
2. `..` path segments;
3. empty path segments;
4. backslashes;
5. NUL or control bytes;
6. duplicate names after normalization;
7. symlinks, hardlinks, and special files;
8. encrypted entries;
9. ZIP64 entries at 1.0;
10. nested ZIP archives;
11. unsupported compression methods;
12. invalid central directory or local header mismatch;
13. CRC mismatch;
14. manifest digest mismatch;
15. size and compression ratio overflow.

---

## 6. Resource Limits

The reference implementation exposes a single `ResourceLimits` source of truth.
Every parser, validator, package reader, renderer, and projection uses it.

| Parameter | Default limit | Code |
|---|---:|---|
| Source bytes, single Text NODX | 64 MiB | `NODX-E012` fatal |
| Front matter bytes | 64 KiB | `NODX-E012` fatal |
| Line length | 1 MiB | `NODX-E012` error |
| Single attribute value | 64 KiB | `NODX-E012` error |
| ID length | 256 UTF-8 bytes | `NODX-E012` error |
| Block nesting depth | 32 | `NODX-E012` error |
| Inline nesting depth | 32 | `NODX-E012` error |
| Nodes per document | 100,000 | `NODX-E012` fatal |
| Data URI size | 5 MiB | `NODX-E012` error |
| Expanded AST memory estimate | 64 MiB | `NODX-E012` fatal |
| Include depth | 8 | `NODX-E011` or `NODX-E012` |
| Package uncompressed size | 256 MiB | `NODX-E012` fatal |
| Package file count | 1,024 | `NODX-E012` error |
| Package single entry size | 64 MiB | `NODX-E012` error |
| Package compression ratio, per entry | 100:1 | `NODX-E012` fatal |
| Package nested ZIP depth | 0 | `NODX-E010` fatal |
| URL length | 4 KiB | `NODX-E020` error |
| Manifest entries | 1,024 | `NODX-E012` error |

Host policy may tighten these limits. It must not relax them in the reference
implementation without a documented ceiling and tests.

---

## 7. Workspace Target

### 7.1 Current workspace

```text
crates/
  nodx-core/
  nodx-cli/
packages/
  nodx-js/
spec/tests/conformance/
examples/
scripts/
apps/desktop/
```

### 7.2 Required by 1.0

```text
crates/
  nodx-core/          # AST, parser, canonical JSON, ResourceLimits, navigation graph
  nodx-validate/      # semantic validation and profile support
  nodx-url/           # URL parsing, normalization, ResourcePolicy
  nodx-package/       # safe package reader, manifest verifier, virtual FS
  nodx-style/         # NODS allowlist parser or safe rejector
  nodx-ncp/           # NCP semantic projection
  nodx-render-html/   # safe HTML renderer
  nodx-cli/           # CLI facade
packages/
  nodx-js/            # independent parser, canonical JSON, NCP semantic
spec/
  grammar/
  schemas/
  tests/
    conformance/
    negative/
    security/
    golden/
fuzz/
```

### 7.3 Deferred workspace crates

These are not required for 1.0:

```text
crates/
  nodx-cst/
  nodx-sign/
  nodx-agent-sdk/
  nodx-render-docx/
  nodx-render-pptx/
  nodx-render-pdf-native/
packages/
  nodx-python/
```

### 7.4 Boundary rules

1. `nodx-core` has no renderer, ZIP, crypto, network, async runtime, or
   filesystem dependency.
2. `nodx-url` depends on no other workspace crate.
3. `nodx-validate` depends on `nodx-core` and `nodx-url`.
4. `nodx-package` does not depend on renderers.
5. `nodx-style` does not depend on renderers.
6. `nodx-ncp` depends on `nodx-core` and validation types only when needed.
7. `nodx-render-html` never fetches resources.
8. `nodx-cli` may depend on every crate; no crate depends on `nodx-cli`.
9. No crate uses `unsafe` in code that touches untrusted input.
10. Any dependency added to a security boundary requires a short threat note in
    the pull request.
11. `toc` resolution is implemented once as a pure navigation graph helper
    shared by validation, HTML rendering, and NCP projection.

---

## 8. Milestone Roadmap

Milestones are ordered to reduce security risk and maximize usable releases.

| Milestone | Release | Goal | Blocking for |
|---|---|---|---|
| M0 | pre-1.0 | Contract cleanup and repo hygiene | all |
| M1 | pre-1.0 | Split monolith without behavior change | all implementation work |
| M2 | 1.0 alpha | Validator, profiles, diagnostics, CLI contract | 1.0 |
| M3 | 1.0 alpha | Central URL policy and resource limits | 1.0 |
| M4 | 1.0 beta | Safe YAML and safe package reader | 1.0 |
| M5 | 1.0 beta | NODS safe subset and safe HTML renderer | 1.0 |
| M6 | 1.0 rc | JS parity, NCP semantic, fixture corpora | 1.0 |
| M7 | 1.0 | Release gate, docs, conformance report | 1.0 |
| M8 | 1.1 | Signature profile | 1.1 |
| M9 | 1.1 | Agent mutate profile | 1.1 |
| M10 | 1.2 | Editor CST profile | 1.2 |
| M11 | 1.2 | Presentation and exporters | 1.2 |

### 8.1 Dependency graph

```text
M0 -> M1 -> M2 -> M3 -> M4 -> M5 -> M6 -> M7

After M7:

M8  depends on M7
M9  depends on M7 and may use M8 hashes but must not require trust policy
M10 depends on M7
M11 depends on M7 and optionally M10
```

### 8.2 Parallelism

After M1:

- M2 and M3 may start in parallel if they do not both edit AST types.
- M4 package work may start after the `ResourceLimits` and `nodx-url` API are
  stable.
- M5 may start after `nodx-url` is stable.
- M6 JS work may run continuously, but parity is only judged against committed
  fixture contracts.

Agents must coordinate before editing:

- `crates/nodx-core/src/ast.rs`;
- canonical JSON output;
- error registry;
- CLI exit codes;
- fixture expected outputs.

---

## 9. Detailed Milestone Specs

Each milestone below is designed as one or more pull requests. A coding agent
should implement only the requested milestone unless explicitly told otherwise.

### M0: Contract Cleanup

**Release target.** pre-1.0.

**Goal.** Make the public contract precise before code changes.

**Inputs.**

- `NODX_0.1_Working_Draft.md`.
- `README.md`.
- `IMPLEMENTATION_PLAN.md`.
- This plan.

**Tasks.**

1. Create `NODX_1.0_Working_Draft.md` from the 0.1 draft.
2. Preserve the 0.1 draft as historical input.
3. Replace `0.1` with `1.0` only where the contract is intentionally promoted.
4. Add the profile model from Section 4.
5. Add the navigable document model from Section 4.3.
6. Add the minimal viable reader rules.
7. Add resource limits from Section 6.
8. Add CLI exit code semantics from Section 10.
9. Add unsupported-feature behavior.
10. Add the error registry from Section 13.
11. Update README and `IMPLEMENTATION_PLAN.md` so they do not claim unsupported
    modules.
12. Add `SECURITY.md` and `THREAT_MODEL.md` skeletons.

**Out of scope.**

- No parser changes.
- No renderer changes.
- No new crates.

**Done criteria.**

- The spec clearly distinguishes required 1.0 scope from future profiles.
- README describes the current implementation honestly.
- `IMPLEMENTATION_PLAN.md` no longer conflicts with this roadmap.
- Error registry is defined in one place and cross-referenced.

**Verification.**

```sh
rtk cargo test
rtk sh scripts/run_conformance.sh
```

### M1: Split `nodx-core` Without Behavior Change

**Release target.** pre-1.0.

**Goal.** Decompose the monolith so independent agents can work safely.

**Target module layout.**

```text
crates/nodx-core/src/
  lib.rs
  ast.rs
  diagnostic.rs
  limits.rs
  bytes.rs
  front_matter.rs
  block_parser.rs
  inline_parser.rs
  attrs.rs
  canonical.rs
  package_baseline.rs
  style_baseline.rs
  validate_baseline.rs
  html_baseline.rs
  tui.rs
  ncp_baseline.rs
```

**Tasks.**

1. Move code mechanically.
2. Keep public API compatible.
3. Add `ResourceLimits` with the Section 6 defaults.
4. Keep existing hard-coded behavior unchanged unless moved into limits.
5. Keep baseline modules until their dedicated crates replace them.
6. Keep canonical JSON byte-identical for all current fixtures.

**Out of scope.**

- No new syntax.
- No new error codes.
- No validator expansion.
- No dependency additions unless needed only for module split tooling.

**Done criteria.**

- `lib.rs` is under 200 LOC.
- Existing tests pass.
- Rust/JS canonical AST conformance remains byte-identical.
- Public function names used by the CLI still compile.

**Verification.**

```sh
rtk cargo test -p nodx-core
rtk cargo test
rtk sh scripts/run_conformance.sh
```

### M2: Validator, Profiles, Diagnostics, CLI Contract

**Release target.** 1.0 alpha.

**Goal.** Make validation explicit and profile-aware.

**Tasks.**

1. Create `crates/nodx-validate`.
2. Move semantic validation out of `nodx-core`.
3. Keep parse-time fatal/error diagnostics in `nodx-core`.
4. Implement `Validator`.
5. Implement `ProfileSet`.
6. Implement unsupported required/optional profile diagnostics.
7. Implement `nodx validate --profile <profile>`.
8. Implement `nodx validate --format json`.
9. Implement stable JSON diagnostic array output.
10. Add negative fixtures for every validation-owned error code.
11. Validate `toc` navigation attributes: `role`, `source`, `scope`, `depth`,
    `min-level`, `max-level`, and accessible label defaults.
12. Add a shared deterministic navigation graph resolver without changing the
    canonical AST shape.
13. Add golden diagnostics.
14. Update CLI exit codes to Section 10.

**Validation-owned codes at 1.0.**

- `NODX-E004`
- `NODX-E006`
- `NODX-E007`
- `NODX-E008`
- `NODX-E009`
- `NODX-E013`
- `NODX-E014`
- `NODX-E016`
- `NODX-E022`
- `NODX-E023`
- `NODX-E024`
- `NODX-E025`

**Out of scope.**

- URL parser details beyond calling `nodx-url` once available.
- Full NODS cascade.
- Package internals.
- Signature checks.
- Renderer-specific navigation layout.

**Done criteria.**

- Parser can produce partial AST for non-fatal errors.
- Validator can run independently on a `Document`.
- Unsupported required profile exits code `3`.
- `warning` and `info` do not raise exit code above `0`.
- Every validation code has at least one fixture.
- Invalid navigation attributes and unresolved `toc` scopes are covered by
  negative fixtures.
- Navigation graph resolution is covered by unit tests and reused by later
  renderer/NCP work.

**Verification.**

```sh
rtk cargo test -p nodx-validate
rtk cargo test
rtk sh scripts/run_conformance.sh
```

### M3: URL Policy and Resource Limits

**Release target.** 1.0 alpha.

**Goal.** Centralize all URL and limit enforcement.

**Tasks.**

1. Create `crates/nodx-url`.
2. Implement `ResourcePolicy`.
3. Implement URI classification by kind.
4. Implement package-relative path normalization.
5. Reject dangerous schemes and obfuscation.
6. Reject control characters and backslash path tricks.
7. Add URL security corpus.
8. Migrate validator, renderer, package baseline, and style baseline to
   `nodx-url`.
9. Remove ad-hoc `starts_with("javascript:")`-style safety checks.
10. Ensure every subsystem receives `ResourceLimits`.

**Out of scope.**

- Network fetching.
- URL rewriting.
- Browser policy integration.

**Done criteria.**

- `rg` finds no ad-hoc URL safety checks outside `nodx-url` tests.
- URL corpus passes.
- Unsafe URLs never reach rendered HTML as active links or asset sources.
- Data URI size and MIME restrictions are enforced.

**Verification.**

```sh
rtk cargo test -p nodx-url
rtk cargo test
rtk sh scripts/run_conformance.sh
```

### M4: Safe YAML and Safe Package Reader

**Release target.** 1.0 beta.

**Goal.** Replace the risky input surfaces with tested safe implementations.

**YAML tasks.**

1. Replace line-oriented front matter parsing with an event-driven safe subset.
2. Reject forbidden YAML constructs before deserialization.
3. Preserve unknown metadata fields.
4. Keep Rust and JS canonical AST parity.
5. Add YAML security corpus.

**Package tasks.**

1. Create `crates/nodx-package`.
2. Support stored and deflate entries if a vetted ZIP dependency is selected.
3. Keep ZIP64 forbidden for 1.0.
4. Enforce package limits.
5. Verify manifest entries, size, digest, and CRC.
6. Expose read-only `PackageFs`.
7. Never extract during read.
8. Add package security corpus.
9. Keep package writer deterministic if writer remains in scope.

**Out of scope.**

- Encrypted packages.
- ZIP64.
- Remote includes.
- Signature verification.

**Done criteria.**

- Hostile YAML fixtures produce expected diagnostics.
- Hostile ZIP fixtures are rejected deterministically.
- Package digest mismatch produces `NODX-E021`.
- Two builds of the same package produce byte-identical output if package build
  is included in this milestone.
- Reading a package performs no filesystem writes.

**Verification.**

```sh
rtk cargo test -p nodx-package
rtk cargo test
rtk sh scripts/run_conformance.sh
```

### M5: NODS Safe Subset and HTML Renderer

**Release target.** 1.0 beta.

**Goal.** Make style and HTML safe before any export work.

**Tasks.**

1. Create `crates/nodx-style`.
2. Parse enough NODS to accept the documented 1.0 safe subset.
3. Reject forbidden selectors, at-rules, properties, and functions.
4. Validate all style URLs through `nodx-url`.
5. Emit `NODX-E027` deterministically.
6. Split safe HTML renderer to `crates/nodx-render-html`.
7. Escape by context: text, attribute, URL, and style.
8. Render resolved `toc` nodes as safe navigation landmarks with deterministic
   links and accessible labels.
9. Add CSP for standalone HTML.
10. Add XSS corpus.
11. Add NODS security corpus.

**Important scope rule.**

Full CSS cascade and computed style are not required for 1.0 unless examples or
spec tests require them. The 1.0 requirement is safe acceptance/rejection, not
pixel-perfect styling.

**Out of scope.**

- CSS grid.
- Container queries.
- Animations.
- Transitions.
- JS-driven interactivity.
- DOCX/PPTX/PDF exporters.
- Prescribing left/right navigation placement as a source-format requirement.

**Done criteria.**

- HTML XSS corpus passes 100%.
- Forbidden NODS corpus passes 100%.
- Renderer never embeds unsanitized CSS.
- Renderer never emits executable document content.
- Supported HTML navigation output links only to safe in-document IDs and
  degrades to a deterministic fallback when entries cannot be resolved.
- Existing print examples still render through safe HTML.

**Verification.**

```sh
rtk cargo test -p nodx-style
rtk cargo test -p nodx-render-html
rtk cargo test
rtk sh scripts/run_conformance.sh
```

### M6: JavaScript Parity, NCP Semantic, Fixture Corpora

**Release target.** 1.0 rc.

**Goal.** Prove independent implementation and publish useful corpora.

**Tasks.**

1. Split `packages/nodx-js` into modules.
2. Keep zero runtime dependencies for the 1.0 JS parser.
3. Implement Plain/Core/Rich parser parity for the 1.0 corpus.
4. Implement canonical JSON parity.
5. Implement NCP semantic parity.
6. Add JS diagnostics parity for shared parser/validator errors.
7. Add resolved navigation entries to NCP semantic output for `toc` nodes.
8. Expand conformance fixtures.
9. Add negative fixtures.
10. Add security fixtures.
11. Expand `scripts/run_conformance.sh` to compare:
    - Rust AST vs JS AST;
    - Rust NCP semantic vs JS NCP semantic.
12. Produce `target/conformance-report.json`.

**Out of scope for JS 1.0.**

- Lossless CST.
- Full package reader.
- Signature verification.
- Agent mutation SDK.
- DOCX/PPTX/PDF export.

**Done criteria.**

- Rust and JS canonical AST match byte-for-byte for every conformance fixture.
- Rust and JS NCP semantic match byte-for-byte for every NCP fixture.
- Navigation fixtures cover a primary document TOC and at least one local
  chapter TOC.
- Conformance report is generated.
- Fixtures are small, targeted, and documented.

**Verification.**

```sh
rtk cargo test
rtk sh scripts/run_conformance.sh
node packages/nodx-js/test/conformance.test.mjs
```

### M7: 1.0 Release Gate

**Release target.** 1.0.

**Goal.** Freeze the format and publish a defensible release.

**Tasks.**

1. Freeze `NODX_1.0_Working_Draft.md`.
2. Freeze the canonical AST contract.
3. Freeze error registry and CLI exit codes.
4. Run the full fixture corpus.
5. Run fuzz targets for parser, inline parser, attrs, YAML, URL, package, NODS.
6. Publish `CONFORMANCE.md`.
7. Publish `INTEROP.md`.
8. Publish `SECURITY.md`.
9. Publish `THREAT_MODEL.md`.
10. Publish `MIGRATION-0.1-TO-1.0.md`.
11. Add release notes with known limitations.
12. Start media type registration or document the planned submission process.

**Done criteria.**

- Section 16.1 is fully satisfied.
- Every known limitation is documented.
- Unsupported future profiles fail closed or warn correctly.
- All release artifacts are generated from committed code and fixtures.

**Verification.**

```sh
rtk cargo test
rtk sh scripts/run_conformance.sh
```

Fuzz verification is milestone-specific and may use local or CI-specific
commands documented in `fuzz/README.md`.

### M8: Signature Profile

**Release target.** 1.1.

**Goal.** Add integrity and authenticity without changing the 1.0 format.

**Tasks.**

1. Create `crates/nodx-sign`.
2. Implement JCS-compatible canonicalization or prove existing canonical JSON
   satisfies the frozen contract.
3. Compute `sha256-BASE64URL_WITHOUT_PADDING`.
4. Verify JWS signatures.
5. Support ES256 as mandatory.
6. Support EdDSA as recommended if dependencies are acceptable.
7. Separate cryptographic validity from trust.
8. Add `TrustPolicy` hook.
9. Verify detached and packaged signatures.
10. Add positive and tamper vectors.

**Dependency rule.**

M8 depends on the 1.0 canonical AST, not on lossless CST. CST trivia must never
affect signature verification.

### M9: Agent Mutate Profile

**Release target.** 1.1.

**Goal.** Add validated document mutations for supervised agents.

**Tasks.**

1. Create `crates/nodx-agent-sdk`.
2. Implement operations:
   - insert;
   - replace;
   - delete;
   - add-attribute;
   - set-attribute;
   - remove-attribute;
   - add-comment;
   - approve;
   - reject.
3. Require target resolution by ID, path, or hash.
4. Require `beforeHash` for mutating operations.
5. Compute `afterHash`.
6. Validate after every operation.
7. Roll back failed batches atomically.
8. Emit JSONL change records.
9. Keep LLM API integration out of scope.

**Dependency rule.**

M9 may use signature hashes, but it must not require a trusted signature to
apply a local validated change.

### M10: Editor CST Profile

**Release target.** 1.2.

**Goal.** Support byte-preserving editor workflows.

**Tasks.**

1. Create `crates/nodx-cst`.
2. Preserve source bytes, line endings, delimiters, whitespace, and attribute
   order.
3. Preserve recoverable invalid syntax.
4. Map CST nodes to AST nodes.
5. Implement parse -> emit byte-identical round-trip.
6. Implement local patch primitives.
7. Integrate with M9 for minimal rewrites.

**Dependency rule.**

CST is editor state. It must not change canonical AST hashes.

### M11: Presentation and Exporters

**Release target.** 1.2 or later.

**Goal.** Add credible output targets without weakening the source format.

**Tasks.**

1. Define Presentation Profile.
2. Implement PDF through safe paged HTML host bridge first.
3. Implement DOCX exporter if semantic loss reports are ready.
4. Implement PPTX exporter after Presentation fixtures exist.
5. Emit loss reports for every lossy export.
6. Validate exports with external validators where practical.

**Out of scope until explicitly approved.**

- Native pure-Rust PDF renderer.
- WYSIWYG editor.
- Pixel-perfect DOCX/PPTX round-trip.

---

## 10. CLI Contract

### 10.1 Required 1.0 commands

```sh
nodx inspect file.nodx
nodx ast file.nodx [--format json]
nodx validate file.nodx [--profile plain|core|rich|style|package|agent-read] [--format text|json]
nodx diagnostics file.nodx [--format text|json]
nodx html file.nodx [--standalone] [--csp]
nodx tui file.nodx
nodx ncp file.nodx [--mode semantic]
nodx package inspect file.nodx
nodx package verify file.nodx
```

### 10.2 Deferred commands

These are not required for 1.0:

```sh
nodx ncp file.nodx --mode lossless|summary
nodx package build dir/ -o out.nodx
nodx sign verify file.nodx
nodx sign sign file.nodx
nodx agent apply changes.jsonl file.nodx
nodx agent diff file.nodx changes.jsonl
nodx export pdf file.nodx -o out.pdf
nodx export docx file.nodx -o out.docx
nodx export pptx file.nodx -o out.pptx
```

If any deferred command is present before its milestone, it must be clearly
marked unstable in help output and documentation.

### 10.3 Exit codes

| Code | Meaning |
|---:|---|
| 0 | Success. No `fatal` or `error` diagnostics. |
| 1 | I/O or runtime failure. |
| 2 | Parse, validation, package, URL, style, or security failure. |
| 3 | Required profile or feature unsupported by this CLI build. |

Rules:

1. `fatal` and `error` diagnostics produce exit code `2` unless the specific
   failure is unsupported required capability, which produces `3`.
2. `warning` and `info` diagnostics produce exit code `0`.
3. File read/write errors produce exit code `1`.
4. Unknown CLI command or invalid CLI arguments produce exit code `1`.
5. JSON output must be parseable even when diagnostics exist.

### 10.4 Stable diagnostic JSON

`nodx validate --format json` emits:

```json
[
  {
    "code": "NODX-E024",
    "severity": "error",
    "message": "Required profile is unsupported.",
    "line": null,
    "column": null,
    "target": "profile:signature"
  }
]
```

Fields are always present. Unknown extra fields are not allowed in 1.0.

---

## 11. JavaScript Implementation Contract

The JavaScript implementation is a first-class interop implementation for 1.0.
It is not required to implement every Rust subsystem.

### 11.1 Required for JS 1.0

1. Plain parser.
2. Core parser.
3. Rich parser for the 1.0 conformance corpus.
4. Safe front matter subset.
5. Inline parser parity.
6. Attribute parser parity.
7. Canonical Semantic AST parity.
8. NCP semantic parity.
9. Resolved navigation entries in NCP semantic output for `toc` nodes.
10. Shared diagnostics for parser-owned errors.
11. Resource limits matching Section 6 where applicable.
12. ESM package.
13. Node 20 and modern browser compatibility.
14. Zero runtime dependencies.

### 11.2 Optional for JS 1.0

1. Safe HTML rendering.
2. Stored-entry package sniffing.
3. NODS safe rejection.

If implemented, optional JS behavior must share fixtures with Rust.

### 11.3 Deferred for JS

1. Full package reader.
2. Signature verification.
3. Lossless CST.
4. Agent mutations.
5. DOCX/PPTX/PDF export.

### 11.4 Target JS layout

```text
packages/nodx-js/
  src/
    index.mjs
    ast.mjs
    bytes.mjs
    frontMatter.mjs
    blockParser.mjs
    inlineParser.mjs
    attrs.mjs
    canonical.mjs
    diagnostics.mjs
    limits.mjs
    ncp.mjs
    url.mjs
  test/
    conformance.test.mjs
    canonical.test.mjs
    ncp.test.mjs
  package.json
  README.md
```

---

## 12. Testing and Release Gates

### 12.1 Test layers

1. Unit tests for parser, validator, URL, package, style, renderer.
2. Positive conformance fixtures.
3. Negative fixtures with exact diagnostic codes.
4. Security fixtures.
5. Canonical AST goldens.
6. NCP semantic goldens.
7. HTML safety tests.
8. Package tamper tests.
9. Fuzz harnesses.
10. Cross-platform CI.

### 12.2 Corpus targets for 1.0

| Corpus | Minimum before 1.0 |
|---|---:|
| Conformance positive fixtures | 80 |
| Negative fixtures | 50 |
| URL security inputs | 50 |
| YAML hostile inputs | 30 |
| ZIP hostile archives | 30 |
| NODS hostile inputs | 30 |
| HTML/XSS payload fixtures | 40 |
| Package digest/tamper fixtures | 10 |
| NCP semantic goldens | 20 |
| Navigation/TOC fixtures | 6 |

These numbers are release gates. During earlier milestones, smaller corpora are
acceptable only if the milestone document says so.

### 12.3 Conformance runner

By 1.0, `scripts/run_conformance.sh` must:

1. build Rust CLI;
2. run Rust AST;
3. run JS canonical AST;
4. diff AST byte-for-byte;
5. run Rust NCP semantic;
6. run JS NCP semantic;
7. diff NCP byte-for-byte;
8. write `target/conformance-report.json`;
9. exit non-zero on any mismatch.

### 12.4 Fuzz targets

Required fuzz targets by 1.0:

- byte parser;
- front matter parser;
- block parser;
- inline parser;
- attribute parser;
- URL parser;
- package reader;
- NODS parser or rejector;
- NCP serializer.
- navigation resolver.

Release candidate fuzz budget:

- at least 24 CPU-hours per target on the 1.0 release branch;
- no known panics;
- no known OOMs;
- every accepted finding documented in `SECURITY.md`.

### 12.5 Golden policy

Goldens freeze external contracts only. Do not write goldens for internal helper
shapes unless those helpers are public API.

Frozen goldens at 1.0:

- canonical AST;
- diagnostic JSON;
- NCP semantic;
- selected standalone HTML security outputs.

Not frozen at 1.0:

- pretty HTML formatting;
- TUI formatting;
- internal validation traversal order unless visible in diagnostics.

---

## 13. Error Registry

The registry is frozen at 1.0.

| Code | Default severity | Owner | Description |
|---|---|---|---|
| `NODX-E001` | fatal | `nodx-core` | Invalid UTF-8. |
| `NODX-E002` | fatal | `nodx-core` | U+0000 present. |
| `NODX-E003` | fatal | `nodx-core` | Unterminated front matter. |
| `NODX-E004` | error | `nodx-validate` | Missing or invalid schema for claimed profile. |
| `NODX-E005` | error | `nodx-core` | Unbalanced or mismatched block delimiter. |
| `NODX-E006` | error | `nodx-validate` | Duplicate ID. |
| `NODX-E007` | error | `nodx-validate` | Unresolved reference. |
| `NODX-E008` | error | `nodx-validate` | Unresolvable asset. |
| `NODX-E009` | error | `nodx-validate` | Missing required text alternative. |
| `NODX-E010` | error | `nodx-url`, `nodx-package` | Unsafe path or path traversal. |
| `NODX-E011` | error | `nodx-package` | Include cycle. |
| `NODX-E012` | fatal/error | shared | Resource limit exceeded. |
| `NODX-E013` | warning | `nodx-validate` | Variable referenced but not declared. |
| `NODX-E014` | warning | `nodx-validate` | Custom component not declared. |
| `NODX-E015` | info/warning | renderer | Fallback rendering applied. |
| `NODX-E016` | info | `nodx-validate` | Unknown attribute preserved. |
| `NODX-E017` | info/warning | `nodx-sign` | Signature absent or not verified. |
| `NODX-E018` | fatal/warning | `nodx-core` | Byte Order Mark encountered. |
| `NODX-E019` | fatal | `nodx-core` | Forbidden YAML construct. |
| `NODX-E020` | error | `nodx-url` | Unsafe URL or scheme. |
| `NODX-E021` | error | `nodx-package` | Package digest mismatch. |
| `NODX-E022` | warning | `nodx-validate` | Accessibility issue. |
| `NODX-E023` | warning | `nodx-validate` | Unsupported optional feature. |
| `NODX-E024` | error | `nodx-validate` | Required feature unsupported. |
| `NODX-E025` | error | `nodx-validate` | Table grid invalid. |
| `NODX-E026` | warning | export/ncp | Lossy conversion or projection. |
| `NODX-E027` | error/warning | `nodx-style` | Forbidden NODS construct. |

### 13.1 Coverage rule

Every error code must be emitted by at least one fixture before 1.0. For future
owners such as `nodx-sign`, the 1.0 fixture may assert the correct unsupported
profile behavior instead of actual signature verification.

### 13.2 Severity rule

Default severity is part of the 1.0 contract. Host tools may downgrade only in
documented authoring modes. The reference CLI uses default severity.

---

## 14. Agent Execution Rules

These rules are for coding agents implementing this plan.

### 14.1 Work unit discipline

1. Implement one milestone or one explicitly assigned slice of a milestone.
2. Do not start future profiles while working on a 1.0 milestone.
3. Keep each PR externally verifiable.
4. Add fixtures for every new external behavior.
5. Update docs in the same PR as behavior changes.
6. Do not weaken security policy to make examples pass.
7. Do not silently ignore unsupported required features.
8. Do not change canonical output without updating goldens and migration notes.
9. Do not add dependencies to security boundary crates without justification.
10. Do not refactor unrelated code while fixing milestone issues.

### 14.2 Required agent preflight

Before editing, an agent must read:

1. this plan;
2. the active spec;
3. `README.md`;
4. `IMPLEMENTATION_PLAN.md`;
5. the files it will edit;
6. relevant tests and fixtures.

### 14.3 Required agent output

Every implementation agent final report must include:

- files changed;
- behavior changed;
- fixtures added;
- commands run;
- commands not run and why;
- known follow-up work;
- whether canonical AST output changed.

### 14.4 Stop conditions

An agent must stop and ask for maintainer decision if:

1. a milestone requires changing a frozen 1.0 contract;
2. two valid implementation choices produce different canonical bytes;
3. a security fixture appears wrong but relaxing policy would broaden attack
   surface;
4. dependency choice affects crypto, ZIP, YAML, or URL trust boundaries;
5. required behavior conflicts with the active spec;
6. the work needs destructive git operations.

### 14.5 Review checklist

Reviewers check:

1. Is scope limited to the assigned milestone?
2. Are new public behaviors covered by fixtures?
3. Does canonical JSON remain stable unless intentionally changed?
4. Are diagnostics deterministic?
5. Are resource limits enforced at the boundary?
6. Are unsafe URLs blocked before rendering?
7. Does parser recovery preserve partial AST where expected?
8. Does CLI exit code match Section 10?
9. Do navigation/TOC changes avoid renderer-specific layout requirements?
10. Did README/spec change when behavior changed?
11. Did any dependency broaden the trust boundary?

---

## 15. Documentation Deliverables

Required before 1.0:

1. `NODX_1.0_Working_Draft.md`.
2. `NODX_1.0_Evolution_Plan.md`.
3. `README.md`.
4. `IMPLEMENTATION_PLAN.md`.
5. `SECURITY.md`.
6. `THREAT_MODEL.md`.
7. `CONFORMANCE.md`.
8. `INTEROP.md`.
9. `MIGRATION-0.1-TO-1.0.md`.
10. Per-crate README for every public crate.
11. `fuzz/README.md`.

Required before 1.1:

1. `SIGNATURE_PROFILE.md`.
2. Signature security vectors documentation.
3. Trust policy integration notes.
4. `AGENT_MUTATE_PROFILE.md` if M9 ships in 1.1.

Required before 1.2:

1. `EDITOR_PROFILE.md`.
2. `PRESENTATION_PROFILE.md`.
3. Exporter loss report schema.

---

## 16. Final Definitions of Done

### 16.1 NODX 1.0 Definition of Done

NODX 1.0 is ready only when all of these hold:

1. Plain, Core, and Rich syntax are frozen.
2. Canonical Semantic AST is frozen.
3. Error registry is frozen.
4. CLI exit codes are frozen.
5. Profile declaration and unsupported-feature behavior are implemented.
6. Rust parser passes conformance corpus.
7. JS parser passes conformance corpus.
8. Rust and JS canonical AST match byte-for-byte.
9. Rust and JS NCP semantic match byte-for-byte.
10. Validator emits every 1.0 validation code in fixtures.
11. Required unsupported features emit `NODX-E024`.
12. CLI exits code `3` for unsupported required features.
13. URL security corpus passes.
14. YAML security corpus passes.
15. ZIP security corpus passes.
16. NODS forbidden corpus passes.
17. HTML/XSS corpus passes.
18. Package reader verifies manifest digests.
19. Package reader never extracts during read.
20. Resource limits use a single shared source.
21. Safe HTML renderer escapes by context.
22. Declarative `toc` navigation resolves deterministically in HTML and NCP
    fixtures.
23. No reference crate opens network sockets.
24. Fuzz targets exist and release budget is complete or documented.
25. `SECURITY.md`, `THREAT_MODEL.md`, `CONFORMANCE.md`, `INTEROP.md`, and
    migration docs are published.
26. Known limitations are explicit in release notes.

### 16.2 NODX 1.1 Definition of Done

NODX 1.1 is ready when:

1. 1.0 conformance remains green.
2. Signature profile verifies positive and tamper vectors, if shipped.
3. Trust policy separates crypto validity from trust, if shipped.
4. Agent mutate profile validates and rolls back failed batches, if shipped.
5. New profiles are opt-in and do not alter 1.0 canonical AST.
6. New CLI commands are documented and tested.

### 16.3 NODX 1.2 Definition of Done

NODX 1.2 is ready when:

1. 1.0 and 1.1 conformance remain green.
2. CST round-trip is byte-identical for editor fixtures, if shipped.
3. Local rewrite primitives modify minimal source ranges, if shipped.
4. Presentation profile has fixtures before PPTX export ships.
5. Every lossy export emits a loss report.

---

## 17. Appendix A: Current Gaps

Current gaps relative to 1.0:

| Area | Current status | 1.0 target |
|---|---|---|
| Spec | 0.1 draft exists | 1.0 draft frozen |
| Core crate | monolith | split modules |
| Validation | inside `nodx-core` | `nodx-validate` |
| URL policy | ad-hoc checks | `nodx-url` |
| Resource limits | partial hard-coded | shared `ResourceLimits` |
| YAML | line-oriented parser | safe event subset |
| Package | stored ZIP baseline | safe reader, virtual FS, corpus |
| Style | textual heuristic | safe NODS parser/rejector |
| HTML | safe baseline | split crate, XSS corpus |
| NCP | semantic baseline | `nodx-ncp`, Rust/JS parity |
| Navigation | `toc` placeholder | scoped, styled, deterministic navigation nodes |
| JS | single-file parser | modular package |
| Tests | small conformance set | conformance, negative, security, goldens |
| Fuzz | absent | required targets |
| Docs | README and 0.1 docs | 1.0 release docs |

---

## 18. Appendix B: Deferred Profiles

Deferred does not mean unimportant. It means not required to freeze a trustworthy
1.0.

### 18.1 Signature

Why deferred:

- crypto correctness requires separate review;
- trust policy must not be rushed;
- canonical AST can freeze first.

Earliest release: 1.1.

### 18.2 Agent mutations

Why deferred:

- mutation safety depends on stable validation and hashes;
- rollback semantics need focused tests;
- LLM integration must stay outside the core format.

Earliest release: 1.1.

### 18.3 Editor CST

Why deferred:

- byte-preserving CST is a large implementation surface;
- it must not block readers or validators;
- source rewrite quality is separable from format stability.

Earliest release: 1.2.

### 18.4 DOCX/PPTX/PDF exporters

Why deferred:

- exporters multiply fidelity questions;
- output formats are lossy relative to NODX;
- loss reports and safe HTML must exist first.

Earliest release: 1.2, except PDF via safe HTML host bridge may appear earlier
as an unstable command.

### 18.5 Full NODS cascade

Why deferred:

- safe acceptance and rejection is the 1.0 security requirement;
- computed style is useful but not required for the source format to stabilize;
- cascade correctness needs a dedicated selector and specificity corpus.

Earliest release: 1.1 or 1.2, depending on renderer needs.

---

## 19. Appendix C: Agent Handoff Template

Use this template when assigning a milestone to a coding agent.

````md
## Assignment

Milestone:
Scope:
Files likely involved:
Files off-limits:

## Required reading

- NODX_1.0_Evolution_Plan.md
- NODX_1.0_Working_Draft.md
- README.md
- IMPLEMENTATION_PLAN.md
- Relevant source files
- Relevant tests/fixtures

## Required behavior

- ...

## Out of scope

- ...

## Required tests/fixtures

- ...

## Verification commands

```sh
rtk cargo test
rtk sh scripts/run_conformance.sh
```

## Final report must include

- Files changed
- Behavior changed
- Fixtures added
- Commands run
- Canonical AST changed: yes/no
- Follow-ups
````

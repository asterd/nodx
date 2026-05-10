# NODX 1.0 Evolution Plan

**Purpose.** Precise, agent-executable roadmap to take NODX from the current `nodx/0.1` draft and reference engine to a complete, secure, progressively implementable `nodx/1.0`.

**Audience.** Coding agents (Codex, Claude Code), human maintainers, security reviewers, parser/renderer/editor/exporter implementers, RAG/agent integrators.

**Reading guide.** The plan is normative for the reference implementation in this repository. Sections marked **Status** describe what already exists; sections marked **Target** describe what must exist at `nodx/1.0`. Every phase carries explicit **Inputs**, **Tasks**, **Out-of-scope**, **Done criteria**, **Test artifacts**, **Risk**, and **Dependencies**.

**Core rule.** NODX 1.0 must be **easy to implement partially** and **hard to implement unsafely**. A minimal reader must not need package support, signatures, CST, NODS cascade, PDF/DOCX/PPTX export, or agent mutations.

---

## Table of Contents

1. Product Position
2. Versioning, Compatibility, Stability Pledge
3. Profile Model and Declaration Format
4. Security Baseline and Resource Limits
5. Workspace Layout, Crate Boundaries, Module Decomposition
6. Implementation Phases (P0 → P10)
7. Dependency Graph and Parallelism
8. CLI Surface and Exit Codes
9. JavaScript Parser Scope (`nodx-js`)
10. Testing, Fuzzing, Interop Strategy
11. Error Registry Coverage and Severity Rules
12. Documentation Deliverables
13. 1.0 Definition of Done
14. Execution Guidance for Coding Agents
15. Appendix A — Phase Status Snapshot (current repo)
16. Appendix B — Mapping Plan Phases → Spec Sections
17. Appendix C — Non-Goals at 1.0

---

## 1. Product Position

NODX 1.0 is an **open, text-first, semantic source and exchange format for documents**. It is not a pixel-perfect replacement for PDF, DOCX, ODT, PPTX, or HTML. It is the **authoritative source layer** from which those formats are generated, validated, audited, signed, chunked, and safely processed by software agents.

The value proposition is:

1. readable plain text source;
2. structured semantic AST;
3. safe-by-default rendering;
4. progressive profiles;
5. deterministic canonicalization;
6. package-local assets;
7. stable node addressing;
8. LLM/RAG/agent projections and validated mutations.

NODX 1.0 deliberately ships **no execution model**. Documents are inert.

---

## 2. Versioning, Compatibility, Stability Pledge

### 2.1 Version identifiers

- Source version is declared via the front-matter `schema` field (e.g. `schema: nodx/1.0`).
- The Canonical Semantic AST carries an implicit `schema` derived from the source.
- The Compact Projection carries `"schema": "nodx-ncp/1.0"`.
- Change records carry `"schema": "nodx/change/1.0"`.
- Manifests carry `"manifest_version": "1.0"`.

### 2.2 Stability pledge (frozen at 1.0)

The following surfaces are **frozen** at 1.0 and require a major version bump to change:

1. surface syntax for Plain, Core, Rich;
2. Canonical Semantic AST JSON shape and JCS-sorted serialization rules;
3. NCP `lossless` and `semantic` mode JSON shapes;
4. error code numbers and severity assignments listed in §11;
5. JWS protected header `typ` and `cty`;
6. package layout requirements (entry list, mimetype rules, manifest required fields);
7. CLI exit code semantics (§8).

The following surfaces are **unstable** at 1.0 and may evolve in 1.x minor versions:

1. NCP `summary` mode heuristics;
2. validator severity for `warning`/`info` codes;
3. renderer output bytes (only the canonical AST is frozen, not rendered HTML/PDF/DOCX/PPTX);
4. CLI subcommand flags marked `--unstable-*`.

### 2.3 Canonical AST versioning

The canonical bytes produced by `nodx-sign`'s canonicalizer are the contract that signatures depend on. If the canonical serialization rules change, the major version MUST be bumped. A `canonicalVersion: "1.0"` marker is embedded in JWS protected headers so signers and verifiers can detect mismatch deterministically.

---

## 3. Profile Model and Declaration Format

### 3.1 Profile list

Implementations declare exactly which profiles they support. No implementation is required to support all profiles.

| Profile | Required for 1.0 ecosystem | Scope |
|---|---:|---|
| `NODX-Plain-1.0` | yes | UTF-8, paragraphs, headings, safe escaping, implicit metadata. |
| `NODX-Core-1.0` | yes | front matter, blocks, attrs, lists, inline base, diagnostics. |
| `NODX-Rich-1.0` | yes | tables, figures, images, math, footnotes, bibliography, forms, media fallbacks. |
| `NODX-Style-1.0` | yes | full NODS parser, allowlist, cascade, computed style. |
| `NODX-Package-1.0` | yes | ZIP package, manifest, assets, digest verification, safe includes. |
| `NODX-Agent-1.0` | yes | NCP, stable IDs, hashes, validated mutations, change records. |
| `NODX-Signature-1.0` | yes | canonical AST, manifest signing, JWS verification. |
| `NODX-Editor-1.0` | yes | lossless CST, source maps, local rewrites. |
| `NODX-Presentation-1.0` | optional | deck/slide/speaker-notes export semantics. |

A Rich implementation MUST also satisfy Core. A Package implementation MUST at least understand Core parsing for its entry document. Style, Agent, Signature, Editor, and Presentation are orthogonal and combinable.

### 3.2 Declaration format

A document MAY declare which profiles its content **requires** and which it **uses optionally**. Declaration lives in front matter:

```yaml
schema: nodx/1.0
profiles:
  requires: [core, rich]
  optional: [style, signature]
```

Normative rules:

1. `profiles.requires` and `profiles.optional` are arrays of profile short names (`plain`, `core`, `rich`, `style`, `package`, `agent`, `signature`, `editor`, `presentation`).
2. A processor MUST emit `NODX-E024` (severity `error`) when a profile listed under `requires` is not supported.
3. A processor MUST emit `NODX-E023` (severity `warning`) when a profile listed under `optional` is not supported.
4. In a Packaged document, the package manifest `profiles` field has the same shape and MUST be a superset of the entry document's `profiles.requires`.
5. When a document omits `profiles`, the processor infers requirements from used features and applies the same fail-closed rules.

### 3.3 Minimum viable safe viewer

A minimal viewer is conformant if it:

1. implements `NODX-Plain-1.0` or `NODX-Core-1.0`;
2. renders fallback children for unknown/custom blocks;
3. escapes all output per context;
4. blocks unsafe URL schemes (§4.5);
5. never executes document content;
6. never fetches remote resources by default;
7. emits structured diagnostics for unsupported `profiles.requires` features;
8. exits with code `3` (§8) if a required feature is unsupported.

---

## 4. Security Baseline and Resource Limits

NODX 1.0 security defaults MUST be **fail-closed**. Every processor handling untrusted input MUST enforce every rule below or document a host-policy override.

### 4.1 Default policy table

```text
embedded scripts:               forbidden
macros:                          forbidden
plugins:                         forbidden
remote resource loading:         forbidden
file:// references:              forbidden
javascript:/vbscript: refs:      forbidden
remote styles/imports/includes:  forbidden
active SVG:                      forbidden
MathML active/foreign content:   forbidden
media autoplay:                  forbidden
package extraction to disk:      forbidden
unknown component execution:     forbidden
unknown component fallback:      allowed
canonical AST execution:         forbidden
agent mutations w/o validation:  forbidden
```

### 4.2 Required enforcement points

Every processor MUST enforce, in order:

1. UTF-8 validation (`NODX-E001` fatal);
2. BOM/U+0000 handling (`NODX-E002`/`NODX-E018`);
3. byte/line/node/nesting/memory limits (§4.4);
4. safe front matter (§4.6);
5. package path validation (`NODX-E010`);
6. ZIP bomb controls (§4.7);
7. manifest digest verification (`NODX-E021`);
8. URL policy (§4.5);
9. CSS/NODS allowlist (`NODX-E027`);
10. SVG/MathML sanitization or fallback;
11. HTML/text/attribute/style escaping per context;
12. no filesystem writes during normal reading;
13. no network access unless host policy explicitly enables it.

### 4.3 Filesystem and network policy

- Reading a Text NODX document MUST NOT write any file.
- Reading a Packaged NODX document MUST NOT extract to the filesystem; it exposes a **virtual read-only filesystem** to consumers.
- No subsystem in the reference implementation MAY open a network socket. The HTTP/HTTPS surface is reserved for explicitly opted-in host integrations outside this codebase.
- Renderers MUST treat every URL as if it were untrusted and apply the URL policy below.

### 4.4 Resource limits (normative defaults)

These defaults are the contract; host policy MAY tighten them, never relax beyond a documented ceiling.

| Parameter | Default limit | Code on overflow |
|---|---:|---|
| Source bytes (single `.nodx` file) | 64 MiB | `NODX-E012` fatal |
| Front matter bytes | 64 KiB | `NODX-E012` fatal |
| Line length | 1 MiB | `NODX-E012` error |
| Single attribute value | 64 KiB | `NODX-E012` error |
| ID length | 256 UTF-8 bytes | `NODX-E012` error |
| Block nesting depth | 32 | `NODX-E012` error |
| Inline nesting depth | 32 | `NODX-E012` error |
| Nodes per document | 100,000 | `NODX-E012` fatal |
| Data URI size | 5 MiB | `NODX-E012` error |
| Expanded AST memory (Rust `Document`) | 64 MiB | `NODX-E012` fatal |
| Include depth | 8 | `NODX-E011` / `NODX-E012` |
| Package uncompressed size | 256 MiB | `NODX-E012` fatal |
| Package file count | 1,024 | `NODX-E012` error |
| Package single entry uncompressed | 64 MiB | `NODX-E012` error |
| Package compression ratio (per entry) | 100:1 | `NODX-E012` fatal |
| Package nested ZIP depth | 0 (no nesting) | `NODX-E010` fatal |
| URL length (absolute or relative) | 4 KiB | `NODX-E020` error |
| Manifest entries | 1,024 | `NODX-E012` error |

Limits are normative; the reference implementation reads them from a single `ResourceLimits` struct exposed by `nodx-core` and consumed by every other crate.

### 4.5 URL policy

Centralized in `nodx-url`. Every reference (`href`, `src`, asset URL, font URL, include target, NODS `url(...)`, NODS `@import`) MUST be classified and validated by `nodx-url`. Ad-hoc string checks are forbidden.

Allowed schemes by reference kind (default):

| Reference kind | Allowed schemes |
|---|---|
| `link` | `https`, `http`, `mailto`, `tel`, package-relative |
| `asset` (image, audio, video, embed, font) | `data:` (size-limited), package-relative |
| `style` (NODS `@import`, `url()`) | package-relative |
| `include` | package-relative |
| `media-fallback` | package-relative |

Forbidden everywhere unless host policy explicitly enables: `javascript:`, `vbscript:`, `file:`, `jar:`, `chrome:`, `about:`, percent-encoded scheme bypasses (e.g. `%6Aavascript:`), `data:text/html`, `data:application/xhtml+xml`.

### 4.6 Front-matter YAML safe subset (binding)

Allowed: mapping, sequence, plain scalar, double-quoted scalar, single-quoted scalar, block literal scalar (`|`), block folded scalar (`>`).

Forbidden (each fatal `NODX-E019`):

1. anchors `&`;
2. aliases `*`;
3. explicit tags `!!...`, `!<...>`;
4. merge keys `<<`;
5. duplicate mapping keys;
6. multiple documents (`---` more than once before content);
7. non-string mapping keys;
8. non-finite numbers (`NaN`, `+.inf`, `-.inf`);
9. binary/octal/hexadecimal integer specials beyond YAML 1.2 core schema;
10. timestamps as YAML-native types (must be quoted strings).

### 4.7 ZIP safety

Beyond §4.4 limits, the package reader MUST:

1. reject entries whose normalized path is absolute, contains `..`, contains backslashes, or contains NUL/control bytes;
2. reject duplicate entry names after Unicode NFC normalization;
3. reject symlink, hardlink, or special-file entries;
4. reject encrypted entries (no encryption profile at 1.0);
5. reject entries whose declared size, declared CRC, or declared compression method are inconsistent with the central directory;
6. reject ZIP64 only when needed for entries above 4 GiB — at 1.0, **ZIP64 is forbidden**;
7. reject nested ZIP archives as package entries;
8. reject `Zip Slip` patterns via fully normalized path comparison against an allowed prefix list (`assets/`, `styles/`, `components/`, `i18n/`, `keys/`, `signatures/`, `history/`, `media/`, plus the entry document, `mimetype`, `manifest.yaml`, `META-INF/`).

### 4.8 SVG/MathML

Inline SVG and MathML are forbidden at 1.0. SVG and MathML are referenced only as opaque assets and rendered through a sanitized profile in a future minor version. At 1.0 the renderer either:

1. emits a fallback (`alt`, caption, or component children); or
2. embeds the asset as a referenced file with `sandbox` semantics handled by the host viewer.

### 4.9 Security deliverables for 1.0

Before tagging `nodx/1.0`:

1. `spec/tests/security/zip` (≥ 30 hostile archives);
2. `spec/tests/security/url` (≥ 50 URL inputs);
3. `spec/tests/security/nods` (≥ 30 forbidden CSS constructs);
4. `spec/tests/security/svg` (≥ 10 hostile SVG samples used as fallback assets);
5. `spec/tests/security/yaml` (≥ 30 hostile front-matter samples);
6. `spec/tests/security/html` (≥ 40 XSS payloads embedded in safe nodes);
7. `fuzz/` targets for: byte parser, front-matter parser, inline parser, attribute parser, NODS parser, package reader, manifest parser, NCP serializer;
8. Public `SECURITY.md` describing scope, threat model, disclosure channel, and supported versions.

---

## 5. Workspace Layout, Crate Boundaries, Module Decomposition

### 5.1 Status (current repo)

```
crates/
  nodx-core/   <- single crate, 3,134-line lib.rs monolith
  nodx-cli/    <- minimal CLI facade
packages/
  nodx-js/     <- single-file parser, 489 lines (Plain + Core subset + minimal Rich)
spec/tests/
  conformance/ <- 5 fixtures
  golden/      <- empty
examples/      <- 8+ documents including i18n, print, packaged
scripts/       <- conformance runner, package builder
apps/
  desktop/     <- Python viewer
  web/         <- HTML playground
```

### 5.2 Target workspace at 1.0

```
crates/
  nodx-core/          # Semantic AST, parser, canonical JSON, ResourceLimits
  nodx-cst/           # Lossless CST, source maps, local rewrites
  nodx-validate/      # Profile validators, error registry coverage
  nodx-url/           # URI parser, resolver, ResourcePolicy
  nodx-package/       # ZIP reader/writer, manifest verifier, virtual FS
  nodx-style/         # NODS lexer, parser, allowlist, cascade, computed style
  nodx-sign/          # Canonicalization, hashes, JWS verification
  nodx-agent-sdk/     # Operations, change records, batches, transactions
  nodx-ncp/           # Compact projection (lossless/semantic/summary), chunking, hashes
  nodx-render-html/   # Safe HTML renderer
  nodx-render-pdf/    # PDF bridge/exporter (paged HTML by default)
  nodx-render-docx/   # DOCX exporter
  nodx-render-pptx/   # PPTX exporter
  nodx-cli/           # CLI facade
packages/
  nodx-js/            # Independent parser + renderer (Plain/Core/Rich + NCP semantic)
  nodx-python/        # Optional Python bindings (deferred to 1.x)
spec/
  grammar/
  schemas/
  tests/
    conformance/      # ≥ 80 positive fixtures
    negative/         # ≥ 50 invalid fixtures
    golden/           # Canonical AST, NCP, HTML, computed-style goldens
    security/         # ≥ 200 hostile inputs across all subsystems
    loss/             # Loss report goldens
fuzz/                  # Cargo-fuzz / JS fuzz harnesses
apps/
  desktop/
  web/
```

### 5.3 Boundary rules (must hold at 1.0)

Each rule is checked in CI via `cargo deny` on dependency declarations and `cargo udeps` on feature flags.

1. `nodx-core` MUST NOT depend on renderers, network, ZIP, crypto, LLM APIs, regex with backtracking, async runtime, or filesystem.
2. `nodx-validate` depends on `nodx-core` and `nodx-url`; nothing else.
3. `nodx-url` MUST NOT depend on any other workspace crate.
4. `nodx-package` MUST NOT extract to disk by default and MUST NOT depend on renderers.
5. `nodx-style` MUST NOT depend on renderers; it produces a computed-style model consumed by renderers.
6. `nodx-render-html` MUST NOT fetch network resources and MUST NOT depend on `nodx-sign` or `nodx-agent-sdk`.
7. `nodx-agent-sdk` MUST NOT call LLM APIs; it produces structured operations only.
8. `nodx-sign` signs canonical semantic data, never visual output, never CST trivia.
9. `nodx-cli` depends on every crate; no other crate depends on `nodx-cli`.
10. No crate may use `unsafe` in code that touches untrusted input without an explicit, reviewed safety comment.

### 5.4 Internal split of `nodx-core` (Phase P0.5 deliverable)

The monolith MUST be split into the following files inside `crates/nodx-core/src/` before any other phase begins. This is a precondition for parallel work.

```
crates/nodx-core/src/
  lib.rs               # re-exports, top-level types
  limits.rs            # ResourceLimits, counters
  diagnostic.rs        # Diagnostic, severity, codes
  ast.rs               # Document, Node, Inline, Attrs, Value
  bytes.rs             # parse_bytes, is_packaged_nodx, BOM/UTF-8 handling
  front_matter.rs      # YAML safe subset parser
  block_parser.rs      # delimited blocks, headings, paragraphs, lists, tables
  inline_parser.rs     # inline tokens, escapes, refs, variables, inline math
  attrs.rs             # attribute block parser
  canonical.rs         # canonical JSON (JCS-style, stable for 1.0)
  ncp_semantic.rs      # baseline NCP semantic projection (kept until P8 split)
  render_html.rs       # baseline safe HTML renderer (kept until P9 split)
  render_tui.rs        # baseline TUI renderer
  package_read.rs      # baseline stored-ZIP reader (kept until P3 split)
  url_baseline.rs      # baseline URL safety checks (kept until P2 split)
  validate_baseline.rs # baseline validator (kept until P1 split)
```

Each `*_baseline.rs` is migrated and deleted when its dedicated crate lands. Until then, the public API exposed by `nodx-core` is the same as today, with no behavioral regression on existing fixtures.

---

## 6. Implementation Phases

Phases are ordered by **security surface** and **dependency**, not by user-visible value. Done criteria are objective and CI-checkable.

Severity legend:

- **R1** = release-blocking for 1.0.
- **R2** = release-blocking for 1.0 but parallelizable with R1.
- **S** = post-1.0 stretch; tracked but not blocking.

### Phase P0 — Spec and Contract Cleanup (R1)

**Goal.** Make the public contract precise before adding code.

**Inputs.** Current Working Draft, current README, current implementation plan.

**Tasks.**

1. Replace `0.1` markers with `1.0` in the spec where appropriate, while preserving `0.1` as a historical reference.
2. Add an explicit **Minimum Viable Reader** subsection in the spec (mirrors §3.3 of this plan).
3. Document **unsupported-feature behavior** in the spec: required features fail closed with `NODX-E024`; optional features emit `NODX-E023` and degrade.
4. Pin the **profile declaration format** (§3.2 of this plan) in the spec.
5. Publish **error code ownership and severity rules** (§11 of this plan) as a normative table.
6. Publish the **media-type registration plan** with a target IANA submission window.
7. Update README and `IMPLEMENTATION_PLAN.md` to remove any claim of unsupported modules.

**Out-of-scope.** No code changes other than doc cross-references.

**Done criteria.**

1. README, `IMPLEMENTATION_PLAN.md`, and the spec do not claim unsupported modules.
2. Every profile has explicit MUST/SHOULD/MAY behavior at 1.0.
3. `nodx validate --profile <profile>` has a defined input/output contract documented in the CLI section of the spec.
4. The error registry table in the spec matches §11 of this plan byte-for-byte.

**Test artifacts.** None (documentation phase).

**Risk.** Low.

**Dependencies.** None.

---

### Phase P0.5 — Internal Split of `nodx-core` Monolith (R1)

**Goal.** Decompose `crates/nodx-core/src/lib.rs` (3,134 LOC monolith) into named modules so subsequent phases can land independently.

**Inputs.** Current `nodx-core` source.

**Tasks.**

1. Create the file layout in §5.4.
2. Move existing functions into the listed files without behavioral changes.
3. Introduce `ResourceLimits` as a struct with the §4.4 defaults; thread it through `parse_bytes`.
4. Promote all hard-coded constants for limits into `limits.rs`.
5. Make every existing public symbol re-exported from `lib.rs` so downstream code (CLI, JS conformance script) continues to work.
6. Add module-level `#![forbid(unsafe_code)]` where possible.

**Out-of-scope.** No new behavior. No new error codes.

**Done criteria.**

1. `lib.rs` is under 200 LOC and contains only re-exports and top-level wiring.
2. Every previously passing test still passes.
3. Conformance Rust↔JS still matches byte-for-byte.
4. CI step `cargo test -p nodx-core` runs each module file independently for unit tests.

**Test artifacts.** No new fixtures; existing tests must remain green.

**Risk.** Medium (large mechanical refactor; risk of regressing canonical JSON byte output).

**Dependencies.** P0.

---

### Phase P1 — Validation as a First-Class Module (R1)

**Goal.** Separate semantic validation from parsing; expand error code coverage.

**Inputs.** P0.5 split, error registry in §11.

**Tasks.**

1. Create `crates/nodx-validate` with `Validator` API: `validate(&Document, &Profiles, &ResourceLimits) -> Vec<Diagnostic>`.
2. Move `validate_*` functions out of `nodx-core` into `nodx-validate`. `nodx-core` retains only fatal parse-time errors (E001, E002, E003, E005, E012, E018).
3. Implement validators for Core and Rich profile.
4. Expand error coverage from the current 14 emitted codes to the full set in §11 that applies to validation phase: `NODX-E006`, `E007`, `E008`, `E009`, `E013`, `E014`, `E015`, `E016`, `E022`, `E023`, `E024`, `E025`, `E026`.
5. Implement profile support matrix: validator receives the host's supported-profile list and emits `NODX-E024` (required missing) or `NODX-E023` (optional missing) accordingly.
6. Add JSON diagnostic output mode: `nodx validate --format json` produces a stable JSON array of diagnostics with the shape defined in spec §5.1.
7. Ensure validators are composable: callers can run only a subset of validators (e.g., accessibility only).

**Out-of-scope.** URL policy details (P2). NODS validation details (P5). Package validation details (P3). Signature validation details (P7).

**Done criteria.**

1. Every error code in §11 with phase `validate` is emitted by at least one negative fixture.
2. Invalid fixtures fail validation deterministically across Rust and JS parsers.
3. Parsers still produce a partial AST when only `error` (not `fatal`) diagnostics occur.
4. `nodx validate --format json` output is golden-tested.
5. Validator can be run with an empty supported-profile list and reports every used profile feature as `NODX-E024`.

**Test artifacts.**

- ≥ 30 negative fixtures under `spec/tests/negative/validate/`.
- Golden JSON diagnostics under `spec/tests/golden/diagnostics/`.

**Risk.** Low.

**Dependencies.** P0.5.

---

### Phase P2 — URL Resolver and Resource Policy (R1)

**Goal.** Centralize URL handling and resource classification.

**Inputs.** §4.5 URL policy.

**Tasks.**

1. Create `crates/nodx-url`.
2. Implement a standards-compatible URI reference parser (RFC 3986 + WHATWG URL where they overlap; default to the stricter rule).
3. Implement `ResourcePolicy` with the table in §4.5.
4. Classify references by kind: `link`, `asset`, `style`, `include`, `font`, `media`.
5. Reject controls (U+0000–U+001F, U+007F), dangerous schemes, percent-encoded scheme bypasses, path traversal, backslashes, mixed-encoding tricks.
6. Normalize package-relative paths: `../`, `./`, redundant slashes, NFC normalization, case-folding only for ASCII drive-letter-style prefixes (forbidden anyway).
7. Enforce offline default. Network access is impossible from this crate.
8. Migrate all callers (`render_html`, validator, package reader) to `nodx-url`. Remove every `starts_with("javascript:")`-style ad-hoc check.
9. Add the URL obfuscation corpus (§10).

**Out-of-scope.** Asset fetching (no network code at 1.0).

**Done criteria.**

1. All renderers, validators, and the package reader call `nodx-url`.
2. No module contains ad-hoc URL safety checks.
3. The URL security corpus passes 100%.
4. Fuzz target `fuzz_targets/url_parse.rs` runs ≥ 1 M iterations without panic.

**Test artifacts.** ≥ 50 fixtures in `spec/tests/security/url/`.

**Risk.** Medium (URL parsers historically harbor subtle bugs).

**Dependencies.** P0.5.

---

### Phase P3 — Safe Package Reader/Writer (R1)

**Goal.** Extend current stored-ZIP reader to a complete, safe Packaged NODX implementation.

**Status.** A baseline stored-only reader and manifest digest checker already exist in `nodx-core` (`parse_package_manifest`, `validate_package_path`). They cover the happy path for the bundled showcase.

**Inputs.** §4.4 limits, §4.7 ZIP safety, package layout from spec §18.

**Tasks.**

1. Create `crates/nodx-package`.
2. Use a mature, well-reviewed ZIP library behind a strict wrapper (the wrapper, not the library, is the trust boundary).
3. Support stored and deflate entries. **ZIP64 is forbidden at 1.0.**
4. Reject unsafe paths, duplicates after NFC normalization, symlinks, hardlinks, special files, encrypted entries.
5. Enforce file count, total size, single entry size, depth, name length, compression ratio (§4.4).
6. Parse manifest with the `nodx-core` YAML safe-subset parser.
7. Verify declared size and SHA-256 for every listed entry. Mismatch is fatal (`NODX-E021`).
8. Expose a read-only **virtual filesystem** abstraction (`PackageFs::read(path) -> Result<&[u8], _>`). Never write to disk during read.
9. Make the package writer deterministic: fixed entry order, zeroed timestamps, fixed external attributes, deterministic deflate level.
10. Add the ZIP security corpus.

**Out-of-scope.** Encryption (deferred). ZIP64 (deferred). Signature verification (P7).

**Done criteria.**

1. Reader never writes to disk during read.
2. ZIP security corpus passes 100%.
3. Two consecutive runs of `nodx package build` on the same source produce byte-identical archives.
4. Manifest digest mismatch produces `NODX-E021` and aborts the document load.
5. Fuzz target `fuzz_targets/package_read.rs` runs ≥ 5 M iterations without panic.

**Test artifacts.** ≥ 30 fixtures in `spec/tests/security/zip/` covering: path traversal, absolute paths, NUL bytes, backslashes, duplicate names, NFC collisions, symlink markers, encrypted entries, oversized entries, zip-bomb ratios, corrupted central directory, mismatched CRC, declared-size vs actual-size mismatch.

**Risk.** High (ZIP parsing is a classic exploitation surface).

**Dependencies.** P0.5, P2 (for URL handling of package-relative paths).

---

### Phase P4 — Safe YAML Front Matter (R1)

**Goal.** Replace the ad-hoc front-matter parser with an event-driven safe subset.

**Status.** Current front-matter parser in `nodx-core` covers simple mappings, sequences, scalars, booleans, numbers, null. It is hand-rolled, line-oriented, and does not produce event-level diagnostics.

**Inputs.** §4.6 YAML safe subset.

**Tasks.**

1. Adopt a YAML 1.2 event parser (e.g., `saphyr-parser` in Rust; `yaml` in JS with the unsafe constructors disabled).
2. Validate events **before** deserialization. The validator runs over the raw event stream and rejects forbidden constructs from §4.6 with `NODX-E019`.
3. Reject aliases, anchors, tags, merge keys, duplicate keys, multiple documents, non-string keys, non-finite numbers, native timestamps.
4. Preserve unknown metadata fields verbatim in `Document.meta` so future profiles can add fields without breaking 1.0 readers.
5. Normalize only YAML event content, not Unicode text (Unicode normalization is the renderer's choice when needed).
6. Add the YAML security corpus.
7. Ensure Rust and JS produce **identical** canonical AST output for every front-matter shape in the conformance corpus.

**Out-of-scope.** Full YAML 1.2 surface (e.g., flow-style merge keys).

**Done criteria.**

1. Every fixture in `spec/tests/security/yaml/` produces `NODX-E019` (fatal) where expected and accepts the documented safe subset where expected.
2. Rust and JS canonical AST remain interoperable on conformance fixtures.
3. Fuzz target `fuzz_targets/front_matter.rs` runs ≥ 5 M iterations without panic.

**Test artifacts.** ≥ 30 fixtures in `spec/tests/security/yaml/`.

**Risk.** Medium.

**Dependencies.** P0.5.

---

### Phase P5 — Full NODS Parser and Cascade (R1)

**Goal.** Replace the current textual NODS lexer with a structured CSS parser, allowlist, cascade, and computed-style model.

**Status.** Current implementation lexes inline `:::style` blocks textually, detects forbidden patterns (`animation`, `:hover`, `position: fixed`, `attr()`, `expression()`, `</style>` breakout), strips offending lines, and emits `NODX-E027`. See [README.md:111](README.md#L111).

**Inputs.** Spec §17 (NODS).

**Tasks.**

1. Create `crates/nodx-style`.
2. Implement a NODS lexer with token kinds: ident, hash, atkeyword, function, string, number, dimension, percentage, url, delim, whitespace, comment, semicolon, comma, colon, lbrace, rbrace, lbracket, rbracket, lparen, rparen.
3. Implement a selector parser for the allowlist in spec §17.2 only: type, class, ID, attribute presence/equality, descendant combinator, child combinator, `:root`, `:first-child`, `:last-child`, `:nth-child(...)`, `:lang(...)`, `:dir(...)`, `:not(...)` over allowed selectors. Reject everything else.
4. Implement a declaration parser with per-property value parsers, driven by the property allowlist in spec §17.3.
5. Support allowed at-rules: `@page`, `@font-face`, package-local `@import`, `@media print`, `@media (color-scheme: …)` restricted to `light` and `dark`.
6. Reject forbidden selectors (e.g. `:hover`, `:focus`, `::before`, `::after` for content injection), forbidden at-rules (`@keyframes`, `@supports`, `@layer` are out at 1.0), forbidden properties, forbidden functions (`expression()`, `attr()` outside of `content`, `url()` with remote schemes), and remote `url()`.
7. Implement cascade order: origin (user-agent < user < author < `!important` reversed), specificity per CSS3 rules with deterministic tie-breakers, source order.
8. Output a **computed-style model**: `ComputedStyle { selector_path, property_name, computed_value, source }` per addressable node.
9. Render goldens for cascade results.
10. Update the safe HTML renderer to consume the computed-style model where it currently embeds raw CSS.

**Out-of-scope.** CSS variables custom properties beyond the `--nodx-*` namespace listed in spec §17. CSS grid. CSS subgrid.

**Done criteria.**

1. Raw CSS is never trusted; every byte passes through the lexer + allowlist.
2. Forbidden NODS emits `NODX-E027` deterministically.
3. Safe mode rejects or drops invalid rules consistently across Rust and JS.
4. Renderers consume computed style for properties they support; unsupported properties pass through as `style="..."` only when the property is in the renderer's safe list.
5. Fuzz target `fuzz_targets/nods.rs` runs ≥ 5 M iterations without panic.

**Test artifacts.**

- ≥ 30 fixtures in `spec/tests/security/nods/`.
- ≥ 20 cascade goldens in `spec/tests/golden/computed-style/`.

**Risk.** High.

**Dependencies.** P0.5, P2 (for `url()` validation).

---

### Phase P6 — Lossless CST and Editor Profile (R2)

**Goal.** Support byte-preserving editor workflows.

**Inputs.** Spec §8.3.

**Tasks.**

1. Create `crates/nodx-cst`.
2. Preserve original bytes or source ranges per token.
3. Preserve line endings, delimiters, whitespace, attribute order.
4. Preserve invalid recoverable tokens so editors can show and repair them.
5. Map every CST node to its corresponding AST node deterministically.
6. Implement a no-op formatter: parse → emit → byte-identical original.
7. Implement local patch primitives: set attribute, remove attribute, insert node, delete node, replace text. Each primitive operates on the minimal CST region.
8. Integrate with `nodx-agent-sdk` (P8) so mutations rewrite source minimally.

**Out-of-scope.** Full structural refactoring (rename-id-everywhere, etc.). Auto-formatting. IDE-side incremental parsing.

**Done criteria.**

1. Parse + no-op rewrite is byte-identical for every conformance fixture.
2. Editing one attribute rewrites only the minimal source region (tested via diff line count: ≤ N lines per single-attribute change, where N depends on attribute kind).
3. Source maps address nodes by ID, path, and byte range.
4. Malformed but recoverable syntax can be re-emitted unchanged so editors can show diagnostics in-place.

**Test artifacts.** Goldens under `spec/tests/golden/cst/` for parse-emit round-trip and minimal-edit diffs.

**Risk.** Medium.

**Dependencies.** P0.5, P1.

---

### Phase P7 — Canonicalization and Signature Profile (R1)

**Goal.** Verify authenticity and integrity without trusting visual output.

**Inputs.** Spec §23, §2.3 of this plan.

**Tasks.**

1. Create `crates/nodx-sign`.
2. Implement JSON Canonicalization Scheme (RFC 8785) rules, or use a vetted crate. The canonical JSON in `nodx-core::canonical` MUST already satisfy JCS by the end of this phase.
3. Canonicalize the Semantic AST **without** diagnostics, source ranges, CST trivia, computed-style results, or renderer state.
4. Canonicalize the package manifest separately.
5. Compute `sha256-BASE64URL_WITHOUT_PADDING` digests.
6. Verify JWS signatures with `ES256` (mandatory) and `EdDSA` (recommended).
7. Support detached and packaged signatures (`signatures/*.jws`).
8. Expose `TrustResult` separately from `CryptoResult`. A valid signature from an unknown key is reported as `cryptoValid: true, trusted: false`.
9. Define an external trust policy hook (`TrustPolicy` trait): `fn check(public_key: &Jwk, alg: Alg, doc_hash: &Sha256) -> TrustDecision`.
10. Embed `canonicalVersion: "1.0"` in the JWS protected header (per §2.3).
11. Drop everything Phase P6 (CST) added during canonicalization; this is the contract that makes signatures stable across editors.
12. Freeze canonical bytes for 1.0; any change requires a major version bump.

**Out-of-scope.** Key management UI. Certificate-transparency integration. DID methods (interface only).

**Done criteria.**

1. Canonical bytes are deterministic and identical across Rust, JS, macOS, Linux, Windows.
2. Tampered AST or tampered package assets fail verification.
3. A valid signature with an untrusted key reports `valid but untrusted`.
4. Signature corpus covers positive, tampered, wrong-key, expired-policy, wrong-`typ`, wrong-`alg`, and `canonicalVersion` mismatch.
5. JCS test vectors from RFC 8785 pass byte-for-byte.

**Test artifacts.**

- ≥ 20 fixtures in `spec/tests/security/sign/`.
- ≥ 5 vectors from RFC 8785 reused under `spec/tests/golden/jcs/`.

**Risk.** High (cryptographic correctness).

**Dependencies.** P0.5, P3, P6.

---

### Phase P8 — NCP and Agent SDK (R1)

**Goal.** Make NODX genuinely differentiated for LLM/RAG/agent workflows; add transactional safety.

**Status.** A baseline NCP semantic projection with deterministic SHA-256 hashes exists in `nodx-core::ncp_json` (semantic mode only). No agent SDK yet.

**Inputs.** Spec §22 (Agent profile, NCP, change records).

**Tasks.**

1. Split NCP into `crates/nodx-ncp`.
2. Implement modes:
   - `lossless`: preserves AST one-for-one (loss array always empty).
   - `semantic`: drops CST trivia and computed style (current behavior).
   - `summary`: lossy, optimized for retrieval; required loss report.
3. For each mode include `sourceHash`, per-node `nodeHash`, per-chunk `chunkHash`, stable paths, IDs, text, attrs, references.
4. Add deterministic chunking by semantic boundaries (section, heading, table, figure, codeblock) with configurable token budgets. Default budget: 1,500 tokens per chunk for `semantic`, 600 for `summary`.
5. Emit a `loss` array for every lossy projection, with one entry per dropped feature.
6. Create `crates/nodx-agent-sdk`.
7. Implement operations: `insert`, `replace`, `delete`, `add-attribute`, `set-attribute`, `remove-attribute`, `add-comment`, `approve`, `reject`.
8. Add **transactional batches**: a `ChangeSet` is a list of operations applied atomically. Either the entire set commits and validates, or none of it is applied. Batch commit emits a single JSONL `change-batch` record that references each child change.
9. Enforce target resolution by ID, path, or hash. Ambiguity is an error.
10. Enforce `beforeHash` (required for mutating ops) and compute `afterHash` post-application.
11. Validate the document after every operation; on failure, roll back the entire batch.
12. Emit JSONL change records into `history/changes.jsonl` (in-memory in non-packaged contexts).
13. Integrate with `nodx-cst` (P6) so mutations rewrite the minimal source region.

**Out-of-scope.** LLM API integration. Prompt templates. Tool-calling protocols.

**Done criteria.**

1. Agent operations cannot silently mutate the wrong node (ID/path/hash mismatch always fails closed).
2. Every mutating operation has `beforeHash` and `afterHash`.
3. Batch failure leaves the document unchanged.
4. Validation failure prevents commit.
5. NCP `semantic` and `lossless` outputs are byte-identical across Rust and JS.
6. RAG indexing can consume NCP directly (documented integration example).

**Test artifacts.**

- ≥ 20 fixtures in `spec/tests/golden/ncp/` (covers all three modes).
- ≥ 15 fixtures in `spec/tests/golden/agent/` (single operations + batches).
- ≥ 10 fixtures in `spec/tests/negative/agent/` (hash mismatch, target ambiguity, post-mutation validation failure).

**Risk.** Medium.

**Dependencies.** P0.5, P1, P6, P7 (for `sourceHash` consistency).

---

### Phase P9 — Renderers and Exporters (R2)

**Goal.** Provide credible output targets while preserving loss reporting. Never compromise the safety contract for fidelity.

**Inputs.** Existing safe HTML renderer; spec §24 (HTML), §26 (interop).

**Tasks.**

1. Before splitting, the existing HTML renderer in `nodx-core::render_html` MUST pass the full XSS security corpus.
2. Split safe HTML renderer to `crates/nodx-render-html`. Consume computed style from `nodx-style`.
3. Generated standalone HTML embeds the CSP from spec §24.1 in a `<meta http-equiv="Content-Security-Policy">` tag.
4. Implement `nodx-render-pdf` via safe HTML paged output first. The renderer uses headless Chromium-style printing via an opt-in host bridge; the crate itself contains no browser binary. A pure-Rust paged renderer is a P9.5 stretch goal.
5. Implement `nodx-render-docx` for semantic text, runs, tables, images, footnotes, headings, lists, citations.
6. Implement `nodx-render-pptx` for the Presentation Profile (slides, speaker notes, slide titles, slide notes).
7. Every lossy exporter MUST emit a machine-readable **loss report** (`exports/<name>.loss.json`).
8. Add golden export tests where output is deterministic (HTML, DOCX, PPTX). PDF is checked via structural smoke tests, not byte equality.
9. Validate exports with external validators where available (HTML via `nu` validator; DOCX via `python-docx` round-trip; PPTX via `python-pptx`).

**Out-of-scope.** WYSIWYG editing. Print preview UI. Font embedding governance (deferred).

**Done criteria.**

1. HTML escapes by context (text, attribute, URL, style) and never executes document content.
2. HTML XSS corpus passes 100%.
3. PDF export works for the existing `examples/print/*` set.
4. DOCX export preserves semantic structure for the conformance corpus.
5. PPTX export preserves slide structure and speaker notes for the presentation fixture set.
6. Every lossy export emits a loss report.

**Test artifacts.**

- ≥ 40 fixtures in `spec/tests/security/html/` (XSS payloads).
- Goldens under `spec/tests/golden/html/`, `spec/tests/golden/docx/`, `spec/tests/golden/pptx/`.
- Loss-report goldens under `spec/tests/loss/`.

**Risk.** Medium (HTML XSS is the highest-frequency external risk).

**Dependencies.** P0.5, P1, P2, P5, P8.

---

### Phase P10 — Interop, Fuzzing, and 1.0 Release Gate (R1)

**Goal.** Prove NODX is independently implementable; pass external security review; freeze the format.

**Inputs.** All prior phases.

**Tasks.**

1. Maintain Rust and JS parsers as independent implementations.
2. The JS parser at 1.0 covers Plain, Core, Rich, NCP `semantic`, NCP `lossless`, and safe HTML rendering. See §9.
3. Publish the conformance fixture corpus with ≥ 80 positive fixtures.
4. Publish the negative-fixture corpus with ≥ 50 invalid fixtures.
5. Publish the security-fixture corpus with ≥ 200 hostile inputs.
6. Run fuzz harnesses (P2, P3, P4, P5, P8) for at least 24 CPU-hours each before tagging.
7. Run cross-platform CI on Linux, macOS, Windows for every release branch.
8. Generate a **public conformance report** comparing Rust and JS outputs across the full corpus.
9. Conduct an external security review and publish the report.
10. Register or start registration for `text/nodx` and `application/nodx+zip` media types with IANA.
11. Freeze `nodx/1.0` syntax, canonical AST, error registry, and the §2.2 stability surfaces.

**Done criteria.**

1. Two parsers emit byte-identical canonical AST for the full conformance corpus.
2. Safe HTML renderer passes the XSS/security corpus 100%.
3. Package reader passes the ZIP security corpus 100%.
4. Validator emits every documented error code in at least one fixture.
5. Documented unsupported features fail closed.
6. Public release notes describe compatibility guarantees, frozen surfaces, and known limitations.
7. Media-type registration is at least filed with IANA.

**Test artifacts.** All prior corpora + the public conformance report.

**Risk.** Low at this point if prior phases land cleanly.

**Dependencies.** All.

---

## 7. Dependency Graph and Parallelism

Phases form this DAG. Edges mean "must complete before".

```
P0  ─►  P0.5  ─┬─►  P1  ─┬─►  P6  ─┐
              ├─►  P2  ─┼─►  P3  ─┼─►  P7  ─►  P8  ─►  P9  ─►  P10
              ├─►  P4  ─┘         │
              └─►  P5  ───────────┘
```

Parallel-safe pairs once P0.5 lands:

- (P1, P2, P4, P5) can be worked on in parallel after P0.5.
- (P3) depends on P2 only.
- (P6) depends on P1 only.
- (P7) depends on P3 and P6.
- (P8) depends on P1, P6, P7.
- (P9) depends on P1, P2, P5, P8.
- (P10) depends on everything.

Agents working in parallel MUST coordinate on the shared `Document` AST: changes to `crates/nodx-core/src/ast.rs` are R1 and must land via small, reviewed PRs.

---

## 8. CLI Surface and Exit Codes

### 8.1 Subcommand list (1.0)

```sh
nodx inspect file.nodx
nodx ast file.nodx [--format json]
nodx validate --profile core file.nodx [--format json]
nodx diagnostics file.nodx [--format json]
nodx html file.nodx [--standalone] [--csp]
nodx tui file.nodx
nodx ncp file.nodx [--mode lossless|semantic|summary] [--budget N]
nodx package inspect file.nodx
nodx package verify file.nodx
nodx package build dir/ -o out.nodx
nodx sign verify file.nodx [--key file.jwk] [--policy file.json]
nodx sign sign file.nodx --key file.jwk [--out file.jws]
nodx agent apply changes.jsonl file.nodx [--out updated.nodx]
nodx agent diff file.nodx changes.jsonl
nodx export pdf  file.nodx -o out.pdf
nodx export docx file.nodx -o out.docx
nodx export pptx file.nodx -o out.pptx
```

Flags marked `--unstable-*` are non-frozen at 1.0 and may change in 1.x.

### 8.2 Exit codes (frozen)

| Code | Meaning |
|---:|---|
| 0 | Success. No `fatal` or `error` diagnostics. |
| 1 | I/O or runtime failure (file not found, permission denied, internal error). |
| 2 | Parse, validation, or security failure (any `fatal` or `error` diagnostic). |
| 3 | A required profile or feature is unsupported by this CLI build. |

The CLI MUST set exit code based on the highest-severity outcome across all subcommand stages. `info` and `warning` never raise the exit code above 0.

### 8.3 Output stability

`nodx ast --format json` output is byte-stable for a given input and CLI version. The hash of this output is the public canonical hash for the document.

---

## 9. JavaScript Parser Scope (`nodx-js`)

### 9.1 Target at 1.0

The JS parser is a **first-class implementation**, not a toy. At 1.0 it MUST cover the majority of NODX documents end-to-end so that browser-based viewers, CI scripts, and embeddable widgets do not need a Rust toolchain.

In scope:

1. **Plain profile** — full.
2. **Core profile** — full.
3. **Rich profile** — full surface: tables (canonical Rich + compact pipe), figures, images, media fallbacks, embeds, footnotes, citations, bibliography, table of contents, page breaks, formulas, read-only forms, language/direction inheritance.
4. **Front matter** — full safe-subset YAML matching P4 rules, sharing fixtures with the Rust parser.
5. **Inline parser** — full: text, strong, emphasis, code, links, spans, refs, variables, inline math, escapes.
6. **Attributes** — IDs, classes, named attributes, quoted values, common attributes (`lang`, `dir`, `id`, `class`, `role`).
7. **Canonical Semantic AST** — byte-identical to Rust on the full conformance corpus.
8. **Diagnostics** — emits the same error codes as Rust where applicable.
9. **Safe HTML rendering** — produces byte-identical HTML to Rust on the conformance corpus for Plain, Core, and Rich (excluding NODS-driven style which is computed style from P5).
10. **NCP semantic and lossless modes** — byte-identical to Rust.
11. **Resource limits** — implements the §4.4 table.

Out of scope at 1.0 (deferred to JS in 1.x):

1. **Lossless CST** (Editor profile) — Rust-only at 1.0.
2. **NODS cascade and computed style** — JS implements the same allowlist and emits `NODX-E027`, but the full cascade is Rust-only at 1.0. JS renders raw allow-listed style declarations into a sanitized `<style>` block; renderer goldens for browser output remain byte-identical to Rust.
3. **Signature verification** — JS verifies JWS in a follow-up; at 1.0 the JS parser exposes the canonical AST so an external verifier can be plugged in.
4. **Package reader** — JS reads stored entries only; full deflate is Rust-only at 1.0 (small enough gap that a single follow-up can close it).
5. **Agent SDK mutations** — JS exposes read-only NCP and validation; mutations are Rust-only at 1.0.
6. **PDF/DOCX/PPTX export** — Rust-only.

### 9.2 Architecture target

`packages/nodx-js/` becomes a small package, not a single file:

```
packages/nodx-js/
  src/
    index.mjs            # public API: parse, canonicalJson, renderHtml, ncp
    bytes.mjs
    frontMatter.mjs
    blockParser.mjs
    inlineParser.mjs
    attrs.mjs
    ast.mjs
    canonical.mjs
    renderHtml.mjs
    ncp.mjs
    url.mjs              # mirrors nodx-url policy
    limits.mjs
    diagnostics.mjs
    nods.mjs             # allowlist lexer for Style fixtures
    package.mjs          # stored-entry ZIP reader
  test/
    conformance.test.mjs
    canonical.test.mjs
    html.test.mjs
    ncp.test.mjs
  package.json           # explicit; no dependencies at 1.0
  README.md
```

The package MUST publish as ESM, work in Node ≥ 20 and modern browsers, and have **zero runtime dependencies** at 1.0.

### 9.3 Interop contract

For every fixture in `spec/tests/conformance/`:

1. `canonicalJson(parse(bytes))` is byte-identical to Rust.
2. `renderHtml(parse(bytes))` is byte-identical to Rust for Plain/Core/Rich.
3. `ncp(parse(bytes), 'semantic')` and `ncp(parse(bytes), 'lossless')` are byte-identical to Rust.

The conformance runner (`scripts/run_conformance.sh`) MUST be expanded to check all four streams above.

### 9.4 Build, packaging, distribution

- The package is published unbundled. No transpilation.
- TypeScript declarations (`*.d.ts`) are emitted by `tsc --emitDeclarationOnly` from JSDoc annotations.
- A browser-ready single-file bundle is produced via a tiny `esbuild` script in `scripts/` and shipped alongside the npm package as `dist/nodx.min.mjs`.

---

## 10. Testing, Fuzzing, Interop Strategy

### 10.1 Test layers (binding)

1. **Unit tests** for parsers, allowlists, policy functions.
2. **Conformance fixtures** for valid syntax (≥ 80 at 1.0). Each fixture targets one syntactic feature.
3. **Negative fixtures** for invalid syntax (≥ 50 at 1.0). Each fixture asserts the exact error code emitted.
4. **Security fixtures** for hostile input (≥ 200 at 1.0).
5. **Golden canonical AST** tests.
6. **Golden NCP** tests (`lossless` and `semantic`).
7. **Golden computed-style** tests.
8. **Golden HTML/DOCX/PPTX** tests where output is deterministic.
9. **Loss-report goldens** for every lossy projection.
10. **Package digest/tamper** tests.
11. **Fuzz harnesses** for byte parser, front matter, inline, attributes, NODS, package, manifest, NCP, URL.
12. **Cross-platform CI** on Linux/macOS/Windows.

Tests assert **external contracts**, not internal structure. A test that only validates an internal field name is a smell.

### 10.2 Conformance runner

`scripts/run_conformance.sh` MUST:

1. Build Rust CLI.
2. For each fixture: run Rust `ast`, JS `canonicalJson`, Rust `html`, JS `renderHtml`, Rust `ncp --mode semantic`, JS `ncp('semantic')`, Rust `ncp --mode lossless`, JS `ncp('lossless')`.
3. Diff each pair byte-for-byte.
4. Exit non-zero on any mismatch.
5. Produce a summary report in `target/conformance-report.json`.

### 10.3 Fuzz budget at 1.0

Each fuzz target runs for at least **24 CPU-hours** on the 1.0 release branch with no panics, OOMs, or new findings. Findings older than the release branch must be fixed or accepted with a public rationale.

### 10.4 Coverage targets

| Surface | Minimum coverage |
|---|---:|
| `nodx-core` block parser | 95% line, 90% branch |
| `nodx-core` inline parser | 95% line, 90% branch |
| `nodx-url` | 95% line, 95% branch |
| `nodx-package` | 95% line, 95% branch |
| `nodx-validate` | 90% line, 80% branch |
| `nodx-style` | 90% line, 80% branch |
| `nodx-sign` | 95% line, 90% branch |

Coverage is informational, not blocking, but a regression below the floor in CI requires reviewer sign-off.

---

## 11. Error Registry Coverage and Severity Rules

### 11.1 Frozen registry

The codes, severities, and phases below are frozen at 1.0. Severity may be downgraded by host policy in narrowly documented cases (e.g. authoring tools allowing `error` to surface non-fatally), but the **default** severity is the contract.

| Code | Default severity | Phase | Description |
|---|---|---|---|
| `NODX-E001` | fatal | parse | Invalid UTF-8. |
| `NODX-E002` | fatal | parse | U+0000 present. |
| `NODX-E003` | fatal | parse | Unterminated front matter. |
| `NODX-E004` | error | validate | Missing or invalid schema for claimed profile. |
| `NODX-E005` | error | parse | Unbalanced or mismatched block delimiter. |
| `NODX-E006` | error | validate | Duplicate ID. |
| `NODX-E007` | error | validate | Unresolved reference. |
| `NODX-E008` | error | validate | Unresolvable asset. |
| `NODX-E009` | error | validate | Missing required text alternative. |
| `NODX-E010` | error | package/url | Path traversal or unsafe package path. |
| `NODX-E011` | error | package | Include cycle. |
| `NODX-E012` | fatal/error | any | Resource limit exceeded. (Severity per limit; see §4.4.) |
| `NODX-E013` | warning | validate | Variable referenced but not declared. |
| `NODX-E014` | warning | validate | Custom component not declared. |
| `NODX-E015` | info/warning | render | Fallback rendering applied. |
| `NODX-E016` | info | validate | Unknown attribute preserved. |
| `NODX-E017` | info/warning | sign | Signature absent or not verified. |
| `NODX-E018` | fatal/warning | parse | Byte Order Mark encountered. |
| `NODX-E019` | fatal | parse | Forbidden YAML construct. |
| `NODX-E020` | error | url | Unsafe URL or scheme. |
| `NODX-E021` | error | package | Package digest mismatch. |
| `NODX-E022` | warning | validate | Accessibility issue. |
| `NODX-E023` | warning | validate | Unsupported optional feature. |
| `NODX-E024` | error | validate | Required feature unsupported. |
| `NODX-E025` | error | validate | Table grid invalid. |
| `NODX-E026` | warning | export/ncp | Lossy conversion or projection. |
| `NODX-E027` | error/warning | style | Forbidden NODS construct. |

### 11.2 Coverage requirement at 1.0

Every code above MUST be emitted by at least one fixture in `spec/tests/`:

- positive fixtures for codes that have a non-failure variant (`E015`, `E016`, `E017`, `E023`, `E026`);
- negative fixtures for everything else.

A CI step `cargo run --bin nodx -- ci-coverage` MUST iterate fixtures and assert this.

### 11.3 Ownership

| Crate | Owns |
|---|---|
| `nodx-core` | E001, E002, E003, E005, E012 (parse-phase), E018 |
| `nodx-url` | E010 (URL form), E020 |
| `nodx-package` | E010 (path form), E011, E012 (package-phase), E021 |
| `nodx-validate` | E004, E006, E007, E008, E009, E013, E014, E016, E022, E023, E024, E025 |
| `nodx-style` | E027 |
| `nodx-sign` | E017 |
| `nodx-render-html` and exporters | E015, E026 |

---

## 12. Documentation Deliverables

At 1.0 the repository MUST publish:

1. The normative spec (`NODX_1.0_Working_Draft.md` — renamed and updated from `NODX_0.1_Working_Draft.md`).
2. This evolution plan (frozen as the historical 1.0 plan; superseded plans archived under `old/`).
3. `SECURITY.md` with threat model, scope, supported versions, disclosure address.
4. `CONFORMANCE.md` describing the corpus structure, the conformance runner, and the public report URL.
5. `INTEROP.md` describing the Rust ↔ JS parity contract.
6. `MIGRATION-0.1-TO-1.0.md` listing every breaking change with a sample patch per change.
7. Per-crate `README.md` describing the crate boundary, public API, and security posture.
8. A short `THREAT_MODEL.md` listing the trust boundaries.

---

## 13. 1.0 Definition of Done

NODX is ready for `nodx/1.0` only when **all** of the following hold:

1. Plain, Core, Rich syntax is frozen.
2. Profile conformance requirements (§3) are explicit and tested.
3. Two independent parsers (Rust, JS) pass the public conformance corpus byte-for-byte.
4. Safe HTML renderer passes the XSS/security corpus 100%.
5. Package reader verifies manifest digests and rejects every fixture in the ZIP security corpus.
6. NODS parser/cascade rejects every forbidden construct in the NODS security corpus.
7. CST can perform byte-identical no-op rewrites for every conformance fixture.
8. Signature verification succeeds on positive vectors and fails on every tamper vector.
9. NCP supports deterministic semantic and lossless chunking; goldens pass on Rust and JS.
10. Agent SDK validates and hashes every mutation; batch failures roll back atomically.
11. Exporters produce loss reports.
12. Unsupported required features fail closed with `NODX-E024` and exit code 3.
13. Every error code in §11 is emitted by at least one fixture.
14. Resource limits (§4.4) are read from a single `ResourceLimits` source across all crates.
15. Media-type registration is at least filed with IANA.
16. External security review findings are resolved or explicitly documented.
17. The §2.2 stability pledge is published.

---

## 14. Execution Guidance for Coding Agents

When implementing this plan:

1. Work one phase at a time per pull request.
2. Respect crate boundaries; do not reach across.
3. Do not add broad abstractions before tests require them.
4. Add one focused fixture per new external behavior.
5. Run `cargo test`, `sh scripts/run_conformance.sh`, and the relevant fuzz target after each phase.
6. Never weaken security policy to make an example pass. If a fixture needs a relaxation, the **fixture** is wrong.
7. Update README/spec when behavior changes, in the same commit.
8. Keep unsupported advanced features explicit (`NODX-E023` / `NODX-E024`), never silently no-op.
9. When in doubt about whether something is a `fatal` or `error`, default to the higher severity and ask in PR review.
10. Prefer small, reviewable PRs over phase-sized ones. A phase landing as 10 PRs is healthier than as one.
11. Before deleting a `*_baseline.rs` file, verify every caller has migrated; a `compile_error!` placeholder is acceptable for one cycle to force a downstream fix.
12. When two phases conflict on `crates/nodx-core/src/ast.rs`, the earlier phase by the §7 DAG wins.

Preferred sequence for solo agents:

```
P0  →  P0.5  →  P1  →  P2  →  P4  →  P3  →  P5  →  P6  →  P7  →  P8  →  P9  →  P10
```

Do not start with exporters. Exporters multiply ambiguity if validation, URL policy, package handling, and canonicalization are not stable.

---

## 15. Appendix A — Phase Status Snapshot (current repo)

Snapshot taken at the date this plan replaces the previous one. Update on every phase landing.

| Phase | Status | Notes |
|---|---|---|
| P0 | partial | Spec and README claim some unsupported features; profile declaration format not yet pinned. |
| P0.5 | not started | `nodx-core` is a 3,134-line monolith. |
| P1 | partial | Validators exist inside `nodx-core` and cover ~14 of the 27 error codes. |
| P2 | partial | URL safety is split across ad-hoc `safe_link_url` / `safe_image_url` checks; no centralized crate. |
| P3 | partial | Stored-ZIP reader and manifest digest verifier exist for the bundled showcase; deflate, full ZIP-bomb controls, and virtual FS are missing. |
| P4 | partial | Hand-rolled front matter parser; YAML safe subset is not enforced at event-stream level. |
| P5 | partial | Textual NODS lexer with allowlist heuristics; no structured parser, no cascade, no computed style. |
| P6 | not started | No CST, no source ranges, no minimal-edit primitives. |
| P7 | not started | Canonical JSON exists; no JWS, no canonicalization rules per JCS, no trust hooks. |
| P8 | partial | NCP `semantic` mode is implemented and used in examples; no `lossless`, no `summary`, no agent SDK, no batches. |
| P9 | partial | Safe HTML and TUI renderers exist; no PDF/DOCX/PPTX exporters; no loss reports. |
| P10 | not started | 5 conformance fixtures, 5 golden fixtures, no fuzz harnesses, no security corpus directories. |

---

## 16. Appendix B — Mapping Plan Phases → Spec Sections

| Plan phase | Spec sections (NODX_0.1_Working_Draft.md) |
|---|---|
| P0 | 1, 4, 5, 28, Appendix A |
| P0.5 | 5, 8 |
| P1 | 5, 8, 11, 25, 28 |
| P2 | 20, 24 |
| P3 | 18, 24 |
| P4 | 7, 24 |
| P5 | 17, 24 |
| P6 | 8.3 |
| P7 | 23 |
| P8 | 22 |
| P9 | 24.1, 26 |
| P10 | 1, 26, 28 |

---

## 17. Appendix C — Non-Goals at 1.0

The following are **explicitly out of scope** at 1.0 and tracked for 1.x or later:

1. Inline SVG and inline MathML.
2. ZIP64 packages.
3. Encrypted packages.
4. Network resource loading (no opt-in flag at 1.0).
5. CSS grid, subgrid, container queries.
6. CSS animations and transitions of any kind.
7. WYSIWYG editing surface.
8. Native PDF renderer (only the HTML-to-PDF bridge ships at 1.0).
9. DID-based trust frameworks (interface only).
10. Streaming parser (single-pass byte-oriented at 1.0).
11. Python bindings (deferred).
12. Real-time collaboration protocol.
13. Server-side template inclusion (no `{{ }}` server expansion; variables are document-local).
14. Macro expansion or any execution model.

Anything not listed above either belongs to 1.0 per the phase plan, or requires an RFC to enter scope.

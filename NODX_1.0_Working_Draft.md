# NODX-WD-0002: NODX 1.0 Working Draft

**Title:** NODX 1.0 - Node-Oriented Document eXchange  
**Document type:** Frozen Working Draft / standards-track technical proposal
**Version:** 1.0 frozen release contract
**Date:** 11 May 2026  
**Derived from:** `NODX_0.1_Working_Draft.md`  
**Primary extension:** `.nodx`  
**Container model:** UTF-8 text or packaged ZIP, detected by bytes  
**Status:** Frozen for NODX 1.0 release-gate verification.

---

## Abstract

NODX is a UTF-8, text-first, semantically structured document format for
portable source documents. A `.nodx` file can be plain text or a ZIP package.
The format is designed for deterministic parsing, safe rendering, explicit
profile support, stable semantic projections, and agent-readable documents.

NODX 1.0 is intentionally smaller than the long-term ecosystem. It defines the
stable text format, canonical Semantic AST contract, NCP semantic projection,
safe package reader contract, profile behavior, resource limits, diagnostics,
and CLI exit semantics. It does not require signatures, lossless CST editing,
agent mutation, native PDF/DOCX/PPTX export, full style cascade, or networked
resource loading.

---

## 1. Status and Scope

This frozen draft promotes only the intentional 1.0 contract from the 0.1
working draft. `NODX_0.1_Working_Draft.md` remains historical input and must
not be rewritten as part of 1.0 cleanup work. Changes to the frozen surfaces in
Section 3 require a future major version update.

Required 1.0 behavior:

1. Plain, Core, and Rich text syntax as defined by this draft.
2. Profile declarations and fail-closed unsupported-feature behavior.
3. Minimal viable reader rules.
4. Declarative `toc` navigation model.
5. Canonical Semantic AST serialization.
6. NCP semantic projection with deterministic hashes.
7. Resource limits.
8. Structured diagnostics and frozen error registry.
9. CLI exit semantics.
10. Safe package reader behavior for `NODX-Package-1.0`.

Deferred profiles and features:

1. `NODX-Agent-Mutate-1.1`.
2. `NODX-Signature-1.1`.
3. `NODX-Editor-1.2` lossless CST and source maps.
4. `NODX-Presentation-1.2`.
5. Full NODS cascade and computed style.
6. Native PDF, DOCX, and PPTX exporters.
7. Browser playground and additional language bindings.

---

## 2. Conformance

The words MUST, MUST NOT, REQUIRED, SHALL, SHALL NOT, SHOULD, SHOULD NOT,
RECOMMENDED, MAY, and OPTIONAL are to be interpreted as described in BCP 14,
RFC 2119, and RFC 8174 when they appear in all capitals.

A conforming processor declares the exact profiles it supports and follows the
security and resource-limit requirements for every untrusted input. A processor
may be a parser, validator, renderer, converter, package reader, projection
tool, or CLI wrapper. A processor is not required to render documents unless it
claims a renderer profile or command.

---

## 3. Versioning

The source schema for 1.0 documents is:

```yaml
schema: nodx/1.0
```

The NCP semantic projection schema is:

```json
"schema": "nodx-ncp/1.0"
```

The package manifest version is:

```json
"manifest_version": "1.0"
```

After 1.0, changes to surface syntax, canonical Semantic AST JSON shape,
canonical JSON serialization, error code numbers and default severities,
required package fields, resource-limit defaults, CLI exit code semantics, or
profile declaration field names require a major version bump.

---

## 4. Profile Model

Profiles are capability declarations, not release milestones. A document may
declare required and optional profiles in front matter:

```yaml
schema: nodx/1.0
profiles:
  requires: [core, rich]
  optional: [style, package]
```

Rules:

1. `profiles.requires` and `profiles.optional` are arrays of short names.
2. Valid 1.0 short names are `plain`, `core`, `rich`, `style`, `package`, and
   `agent-read`.
3. Reserved future short names are `agent-mutate`, `signature`, `editor`, and
   `presentation`.
4. Unsupported required profiles or features produce `NODX-E024` with severity
   `error`.
5. Unsupported optional profiles or features produce `NODX-E023` with severity
   `warning`.
6. A CLI build that cannot satisfy a required profile exits with code `3`.
7. Omitted `profiles` means the validator infers required features from used
   syntax and applies the same rules.
8. Package manifests must declare a profile set that is a superset of the entry
   document's required profiles.

| Tier | Profile | 1.0 status | Scope |
|---|---|---:|---|
| Required format | `NODX-Plain-1.0` | blocking | UTF-8 text, paragraphs, headings, safe escaping, implicit metadata. |
| Required format | `NODX-Core-1.0` | blocking | Front matter, blocks, attrs, lists, inline base, diagnostics. |
| Required format | `NODX-Rich-1.0` | blocking | Tables, figures, images, math text, footnotes, bibliography, forms, declarative navigation/TOC nodes, media fallbacks. |
| Supported subset | `NODX-Style-1.0` | partial blocking | Safe NODS allowlist and rejection. Full cascade is deferred. |
| Supported subset | `NODX-Package-1.0` | package reader only | ZIP package read, manifest, assets, digest verification, no extraction. |
| Projection | `NODX-Agent-Read-1.0` | blocking | Stable IDs, hashes, NCP semantic, read-only agent consumption. |
| Future | `NODX-Agent-Mutate-1.1` | deferred | Validated mutations, batches, change records. |
| Future | `NODX-Signature-1.1` | deferred | JWS verification, trust hooks, signature corpus. |
| Future | `NODX-Editor-1.2` | deferred | Lossless CST, source maps, local rewrites. |
| Future | `NODX-Presentation-1.2` | deferred | Deck/slide/speaker-notes semantics and PPTX export. |

---

## 5. Minimal Viable Reader

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

Package support, signatures, lossless CST, full NODS cascade, PDF/DOCX/PPTX
export, and agent mutation are not required for a minimal reader.

---

## 6. Source Text Model

A `.nodx` file MUST be valid UTF-8. Invalid UTF-8 produces `NODX-E001` with
severity `fatal`. A Byte Order Mark is rejected by strict processors with
`NODX-E018`. U+0000 is rejected with `NODX-E002`.

Input may use LF or CRLF. Canonical serialization emits LF. Source text syntax
is recognized at column 0 unless a specific construct says otherwise.

---

## 7. Resource Limits

The reference implementation exposes one `ResourceLimits` source of truth.
Every parser, validator, package reader, renderer, and projection uses it. Host
policy may tighten these limits. The reference implementation must not relax
them without a documented ceiling and tests.

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

---

## 8. Front Matter and Unsupported Features

Front matter uses the YAML safe subset from the 0.1 draft: mappings, sequences,
strings, finite numbers, booleans, null, and simple flow collections are
allowed. Anchors, aliases, explicit tags, merge keys, duplicate keys, multiple
documents, binary tags, custom objects, and non-finite numbers are forbidden and
produce `NODX-E019`.

Required 1.0 fields for Core documents with front matter:

| Field | Type | Requirement |
|---|---|---|
| `schema` | string | MUST be `nodx/1.0`. |
| `type` | string | MUST be `document` unless a profile defines another root type. |
| `title` | string | REQUIRED for Rich and Package documents; RECOMMENDED for Core. |

Unsupported feature behavior is fail-closed for required capability and
recoverable for optional capability:

1. Unknown or unsupported required profile: `NODX-E024`, `error`, CLI exit `3`.
2. Unknown or unsupported optional profile: `NODX-E023`, `warning`, CLI exit `0`
   unless other errors exist.
3. Unknown attributes are preserved and may produce `NODX-E016`.
4. Unknown custom components must preserve fallback children.
5. Unknown reserved block names without safe fallback are validation errors for
   the claimed profile.

---

## 9. Semantic AST and Document Model

A NODX document is an ordered tree. Each node has a `type`, optional `id`,
ordered `classes`, sorted `attrs`, optional `children`, optional `inlines`, and
optional literal `text`.

The Canonical Semantic AST excludes CST trivia, diagnostics, source byte ranges,
renderer-computed style, and editor state. It preserves node ordering,
attributes, text, custom components, references, and variables. Canonical JSON
serialization is deterministic and uses sorted object keys.

Lossless CST is a deferred Editor profile feature and is not required for 1.0.

---

## 10. Declarative Navigation and `toc`

NODX 1.0 supports navigable documents without prescribing a screen layout. The
format-level feature is a declarative Rich `toc` block with empty source
content. Renderers decide whether a `toc` becomes an outline, menu, print table
of contents, textual fallback, or another host-appropriate representation.

Examples:

```nodx
:::toc {#main-nav role="primary" source="document" depth="2" title="Contents"}
:::

::::section {#chapter-3}
## Chapter 3

:::toc {#chapter-3-nav role="local" scope="#chapter-3" min-level="2" max-level="4" title="In this chapter"}
:::
:::: section
```

Required rules:

1. `toc` is a Rich block node with empty source content.
2. Multiple `toc` nodes are allowed.
3. `role` may be `primary`, `local`, `secondary`, or `breadcrumb`.
4. A global summary uses `source="document"` or omits both `source` and
   `scope`.
5. A local summary uses `scope="#id"` and resolves to the subtree rooted at the
   referenced node.
6. `depth` is a positive integer from `1` to `6` and limits included heading
   levels relative to the selected source.
7. `min-level` and `max-level`, if present, are absolute heading levels from
   `1` to `6`; invalid values or ranges produce `NODX-E004`.
8. `title` is a plain-text accessible label, not rendered inline content. If
   omitted, renderers use a deterministic default label based on `role`.
9. Styling uses normal IDs, classes, attributes, and the Style profile. Source
   documents must not encode fixed left, right, or sidebar layout semantics.
10. Resolved entries are derived from existing `heading` and `section` nodes
    with stable IDs. Generated entries are not duplicated into the canonical AST
    as child nodes.
11. Unresolved `scope="#id"` references produce `NODX-E007`.
12. Renderers that support navigation resolve entries deterministically in
    document order. Renderers that do not support navigation render a safe
    fallback or emit `NODX-E015`.
13. The NCP semantic projection exposes resolved navigation entries so agents
    can understand document structure without renderer-specific HTML.
14. Previous/next navigation is renderer output derived from the resolved
    navigation graph. It is not represented as a source node or as a `toc`
    attribute in NODX 1.0.

The `toc` model is declarative: global and local summaries, role and scope,
depth/min-level/max-level, and styling hooks are source semantics; placement
and previous/next controls are renderer output.

---

## 11. Security Baseline

Processors handling untrusted input fail closed by default.

Forbidden by default:

| Surface | 1.0 rule |
|---|---|
| Embedded scripts, macros, plugins | forbidden |
| Remote resource loading | forbidden |
| `file://`, `javascript:`, `vbscript:` | forbidden |
| Remote styles, imports, includes | forbidden |
| Active or inline SVG | forbidden at 1.0 |
| Inline MathML | forbidden at 1.0 |
| Media autoplay | forbidden |
| Package extraction to disk | forbidden |
| Unknown component execution | forbidden |
| Unknown component fallback | allowed |
| Agent mutations | not part of 1.0 |
| Filesystem writes during read | forbidden |
| Network access during read | forbidden |

Required enforcement order:

1. input byte size;
2. UTF-8 validation;
3. BOM and U+0000 handling;
4. line length and front matter size;
5. front matter safe subset;
6. block and inline parse limits;
7. profile support;
8. path, URL, and package validation;
9. semantic validation;
10. renderer or projection escaping.

---

## 12. CLI Contract

Required 1.0 commands:

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

Deferred commands:

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

Exit codes:

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

Stable diagnostic JSON contains exactly these fields in 1.0: `code`,
`severity`, `message`, `line`, `column`, and `target`.

---

## 13. NCP Semantic Projection

The 1.0 NCP projection is read-only semantic output for agents and indexing. It
is not the authoritative document when NODX source is available.

NCP 1.0 requires:

1. schema `nodx-ncp/1.0`;
2. deterministic source hash;
3. deterministic node paths;
4. stable node IDs where present;
5. deterministic node hashes;
6. semantic text and node kinds;
7. resolved navigation entries for `toc` nodes.

Lossless and summary NCP modes are deferred. Lossy conversion or projection
uses `NODX-E026`.

---

## 14. Error Registry

The registry is frozen at 1.0. Default severity is part of the contract.

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

Every error code must be emitted by at least one fixture before 1.0. For future
owners such as `nodx-sign`, the 1.0 fixture may assert unsupported-profile
behavior instead of actual signature verification.

---

## 15. Current Reference Implementation Status

The repository contains the NODX 1.0 release-gate reference implementation. It
has Rust crates for core parsing, package reading, URL/resource policy,
validation, style safety, HTML rendering, and the CLI, plus an independent
JavaScript parser and semantic NCP projector.

Implemented behavior includes UTF-8 parsing, Plain/Core/Rich syntax, canonical
JSON, diagnostics, safe HTML and TUI rendering, semantic NCP projection, safe
stored-ZIP package reading, URL/resource policy, safe NODS subset validation,
and Rust/JS conformance over the committed fixture corpus.

The known release limitations are tracked in `RELEASE_NOTES-1.0.md`.

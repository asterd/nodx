# Implementation architecture

The reference implementation is a small set of focused crates, each with
a narrow responsibility and a public API surface that can stand alone.
This page is for contributors who need to know where to make a change.

## Crate graph

```
                       ┌──────────────┐
                       │   nodx-cli   │   (CLI entry point)
                       └──────┬───────┘
                              │ depends on
        ┌────────────┬────────┼───────────┬─────────────┬────────────┐
        ▼            ▼        ▼           ▼             ▼            ▼
  nodx-render-html nodx-export nodx-ncp nodx-validate nodx-package nodx-sign
        │            │        │           │             │            │
        └────────────┴────────┼───────────┴─────────────┴────────────┘
                              ▼
                    ┌──────────────────┐
                    │     nodx-core    │   (parser, AST, hashing, navigation)
                    └────────┬─────────┘
                             ▼
                    ┌──────────────────┐
                    │ nodx-url nodx-style │ (shared safety helpers)
                    └──────────────────┘

                    ┌──────────────────┐
                    │     nodx-cst     │   (concrete syntax tree for editors)
                    └──────────────────┘
                    ┌──────────────────┐
                    │   nodx-agent-sdk │   (mutation batches, comments, approvals)
                    └──────────────────┘
```

`nodx-cst` and `nodx-agent-sdk` are extensions that sit alongside the
parsing pipeline. Both consume the AST from `nodx-core` but are otherwise
independent.

## Crate-by-crate

### `nodx-core`

The parser and AST.

- `bytes.rs` — input shape detection (text vs packaged), front matter
  framing.
- `block_parser.rs` — line-based block recognizer; produces `Document`
  with diagnostics. This is where resource limits are enforced.
- `inline_parser.rs` — iterative inline tokenizer. Code/math/var/link
  span handling.
- `attrs.rs` — attribute block parsing and safe-style shorthand
  translation.
- `front_matter.rs` — YAML safe subset; rejects anchors, aliases, tags,
  merge keys, duplicates, multi-doc, timestamps, binary tags, non-finite
  numbers (`NODX-E019`).
- `canonical.rs` — canonical JSON emitter; sorts keys, fixes numeric
  encoding.
- `hashing.rs` — SHA-256, base64url. The single hashing primitive used by
  NCP, signing, and the agent SDK.
- `navigation.rs` — heading graph; powers `::toc` and the docs shell
  layout.
- `tui.rs` — terminal renderer.
- `limits.rs` — `ResourceLimits` struct and defaults.

No `unsafe`. No `dbg!`. No allocations in the inline parser hot path
beyond what the input demands.

### `nodx-url`

URL classification and policy. The single place that decides whether a
URL is safe to reach. Used by the validator, the renderer, and the
package reader.

### `nodx-style`

NODS audit + sanitization. The single place that decides whether a CSS
rule is allowed. Produces `NODX-E027` diagnostics for forbidden rules and
strips them from the rendered output.

### `nodx-validate`

Semantic validation: id uniqueness, profile resolution, attribute shape
enforcement, navigation consistency, reference resolution. Produces the
diagnostics CI relies on. Independent of rendering.

### `nodx-render-html`

The HTML renderer. Emits one self-contained HTML document per call, with
a strict `Content-Security-Policy` `<meta>` header derived from the
inline styles actually used. Themes are baked into the renderer at build
time; the `docs` layout is a structural choice, not a CSS override.

### `nodx-package`

ZIP reader with manifest verification:

- `inflate.rs` — DEFLATE decoder, output bounded by `package_entry_bytes`.
- `manifest.rs` — manifest YAML parser; same safe subset as front matter.
- `lib.rs` — package open, entry validation, SHA-256 digest checks,
  extension resolution.

The reader is read-only and in-memory. It never writes anything to disk
during open or verify.

### `nodx-ncp`

NCP projection. Walks the AST in document order, emits per-node hashes,
exposes the result in `summary`, `chunks`, or `full` modes.

### `nodx-sign`

ES256 (ECDSA P-256, SHA-256) detached JWS verification over the
canonical AST hash. Trust resolution is pluggable; the default trust
policy accepts a small list of pinned public keys.

### `nodx-agent-sdk`

Change batches for agent workflows: `Insert`, `Replace`, `Delete`,
`SetAttribute`, plus comments and approvals. Every operation carries the
expected pre-state hash; applying a batch against a drifted document
fails closed rather than silently re-anchoring.

### `nodx-cst`

Concrete syntax tree. Same parse as `nodx-core`'s AST, plus byte ranges
back to the source. The basis for editor integrations (folding ranges,
inlay hints, selection-aware operations) and for the byte-level patch
operations agents and reviewers can apply.

### `nodx-export`

PDF, DOCX, and PPTX preview exports. Each format produces both the
artifact and a *loss report* — a JSON listing every NODX construct that
did not survive the conversion. Export is opinionated; the loss report
is the contract that lets you tell rich content apart from "everything
shipped".

### `nodx-cli`

The thin command-line wrapper. Reads bytes, dispatches to the right
library, prints the result, exits with the documented code.

## Cross-cutting invariants

Every crate is `#![forbid(unsafe_code)]`. None of them depend on
non-standard-library Rust crates except for the obvious ones (e.g.
`base64` would belong in `nodx-core` but we wrote it inline). The
workspace builds with stable Rust without features, network access, or
codegen.

The parser never allocates per-character. The renderer escapes every
value into its HTML context (text, attribute, URL, style) rather than
trusting block-level escaping. The package reader never extracts to disk.

These are the load-bearing properties of the implementation. Changes
that weaken any of them need a discussion before a PR.

## Where to start a change

| You want to | Touch |
|---|---|
| Add a block | `nodx-core/src/block_parser.rs`, `nodx-render-html/src/lib.rs`, fixture under `spec/conformance/v1.0/`. |
| Add an inline | `nodx-core/src/inline_parser.rs`, `nodx-render-html/src/lib.rs`, fixture. |
| Add a diagnostic | The crate that observes the condition, plus an entry in the [diagnostics reference](../reference/diagnostics.md). |
| Add a profile | `nodx-validate/src/lib.rs` `ProfileSet`, the [profiles reference](../reference/profiles.md). |
| Add a theme | `crates/nodx-render-html/src/lib.rs` (built-in CSS) and `docs/themes/<name>.nods` (source). |
| Add a CLI subcommand | `nodx-cli/src/main.rs`, the [CLI guide](../guide/05-cli.md). |
| Change a default limit | `nodx-core/src/limits.rs` and the [limits reference](../reference/limits.md). |

The conformance package needs to be regenerated whenever the canonical
AST changes. See [conformance](../reference/conformance.md).

# AGENTS.md — Agent & contributor briefing

This file is the entry point for AI coding assistants and human contributors
who land in this repository. It is intentionally short and dense: pointers
to authoritative sources, the invariants you must not break, and a map of
"if you need to change X, touch Y".

> If something in this file conflicts with [`NODX-RFC-0001.md`](./NODX-RFC-0001.md),
> the RFC wins. The RFC is normative; everything else (including this file)
> is informative.

## What NODX is, in three lines

NODX is a text-first, node-oriented document format with a frozen 1.0
contract: a canonical byte-stable AST, a fail-closed security model, a
ZIP-based package for local assets, an agent-readable projection (NCP),
and conformance profiles. It looks like Markdown for the easy parts and
fails closed for everything else.

## Non-negotiable invariants

Touching any of these without an explicit RFC-level discussion is a bug,
not a feature. They appear in CI as hard gates.

1. **Zero `unsafe` Rust.** Every `lib.rs` declares `#![forbid(unsafe_code)]`.
   `nodx-cli/src/main.rs` is the only binary entry point; it should also
   forbid `unsafe` once the lint is added (see [TODO.md](./TODO.md) /
   ongoing audits).
2. **Byte-stable Canonical AST JSON.** Two conforming parsers must produce
   the same bytes for the same input. No `HashMap` in the AST path —
   `BTreeMap<String, _>` everywhere. Hashing goes through
   [`crates/nodx-core/src/hashing.rs`](./crates/nodx-core/src/hashing.rs)
   and nowhere else.
3. **Fail-closed on untrusted input.** URL safety decisions live in
   [`crates/nodx-url`](./crates/nodx-url/). CSS safety lives in
   [`crates/nodx-style`](./crates/nodx-style/). Package safety lives in
   [`crates/nodx-package`](./crates/nodx-package/). Do not duplicate those
   decisions elsewhere; do not bypass them.
4. **No network by default.** No fetch in parser/validator/renderer/package
   reader. The `remote-assets` profile is opt-in and host-policy-gated.
5. **No disk writes during open/parse/validate/render/project.** The
   package reader is read-only and in-memory.
6. **Resource limits enforced before expensive work.** See
   [`crates/nodx-core/src/limits.rs`](./crates/nodx-core/src/limits.rs)
   and the [limits reference](./docs/reference/limits.md). A limit hit
   emits `NODX-E012`, never panics.
7. **Diagnostic codes are a stable contract.** `NODX-Exxx` codes never
   change meaning. The full registry is
   [`docs/reference/diagnostics.md`](./docs/reference/diagnostics.md).
   Adding a code requires both an emitter and a registry entry.
8. **No raw HTML, no script execution, no active SVG/MathML in 1.0.**
   These are degraded to text or rejected. See
   [`SECURITY.md`](./SECURITY.md).
9. **Rust ↔ JavaScript byte-for-byte parity is a release gate.**
   `scripts/run_conformance.sh` must stay green. A divergence is the
   single highest-priority bug class. Python parity exists but is not yet
   gated (see audit findings).

## Crate map — where things live

The reference implementation is 12 focused Rust crates. Cross-crate
graph and per-crate notes:
[`docs/internals/architecture.md`](./docs/internals/architecture.md).

| If you are changing… | Touch this first |
|---|---|
| Block grammar (headings, lists, tables, fences) | [`crates/nodx-core/src/block_parser.rs`](./crates/nodx-core/src/block_parser.rs) |
| Inline grammar (emphasis, code spans, links, variables) | [`crates/nodx-core/src/inline_parser.rs`](./crates/nodx-core/src/inline_parser.rs) |
| Attribute parsing `{key="value"}` | [`crates/nodx-core/src/attrs.rs`](./crates/nodx-core/src/attrs.rs) |
| YAML front matter / safe subset | [`crates/nodx-core/src/front_matter.rs`](./crates/nodx-core/src/front_matter.rs) |
| AST shape / serialization | [`crates/nodx-core/src/{ast,canonical}.rs`](./crates/nodx-core/src/) |
| Resource limits | [`crates/nodx-core/src/limits.rs`](./crates/nodx-core/src/limits.rs) + [`docs/reference/limits.md`](./docs/reference/limits.md) |
| URL classification & policy | [`crates/nodx-url`](./crates/nodx-url/) (single source of truth) |
| CSS/NODS audit | [`crates/nodx-style`](./crates/nodx-style/) (single source of truth) |
| Validation, profiles, id uniqueness | [`crates/nodx-validate`](./crates/nodx-validate/) |
| HTML rendering + CSP | [`crates/nodx-render-html`](./crates/nodx-render-html/) |
| ZIP package reader, manifest, digests | [`crates/nodx-package`](./crates/nodx-package/) |
| NCP projection | [`crates/nodx-ncp`](./crates/nodx-ncp/) |
| Signing (ES256, JWS) | [`crates/nodx-sign`](./crates/nodx-sign/) |
| Agent mutation batches | [`crates/nodx-agent-sdk`](./crates/nodx-agent-sdk/) |
| Concrete syntax tree (editor support) | [`crates/nodx-cst`](./crates/nodx-cst/) |
| Export PDF / DOCX / PPTX + loss reports | [`crates/nodx-export`](./crates/nodx-export/) |
| CLI dispatch & exit codes | [`crates/nodx-cli/src/main.rs`](./crates/nodx-cli/src/main.rs) |
| JS parser parity | [`packages/nodx-js/src/`](./packages/nodx-js/src/) |
| Python parser parity | [`packages/nodx-py/src/nodx/`](./packages/nodx-py/src/nodx/) |
| New diagnostic code | the emitting crate **and** [`docs/reference/diagnostics.md`](./docs/reference/diagnostics.md) |
| New profile | [`crates/nodx-validate`](./crates/nodx-validate/) (`ProfileSet`) + [`docs/reference/profiles.md`](./docs/reference/profiles.md) |
| New theme | [`crates/nodx-render-html`](./crates/nodx-render-html/) (built-in CSS) + `docs/themes/<name>.nods` |
| Conformance fixture | [`spec/conformance/v1.0/fixtures/`](./spec/conformance/v1.0/fixtures/) + regenerate `expected/` |

## Authoritative documents

Read these before making non-trivial changes:

- [`NODX-RFC-0001.md`](./NODX-RFC-0001.md) — normative spec. Sections of
  interest: §6–§12 (syntax), §19 (URL policy), §20 (package), §22
  (limits), §23 (diagnostics), §25 (rendering), §29 (conformance).
- [`SECURITY.md`](./SECURITY.md) — security policy and threat model.
- [`docs/internals/security-model.md`](./docs/internals/security-model.md)
  — implementation-level security boundary.
- [`docs/internals/architecture.md`](./docs/internals/architecture.md) —
  crate graph, cross-cutting invariants.
- [`docs/reference/conformance.md`](./docs/reference/conformance.md) —
  what "passes" means.
- [`docs/IMPLEMENTER_GUIDE.md`](./docs/IMPLEMENTER_GUIDE.md) — pointers
  for third-party implementers.

## Verification gates

Run all of these before claiming a change is done. CI runs the same set;
local green ≈ CI green.

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --release
node --test packages/nodx-js/test/*.mjs
python3 -m pytest packages/nodx-py/tests -q
sh scripts/run_conformance.sh          # Rust ↔ JS parity, byte-for-byte
sh scripts/verify_conformance_package.sh
```

Fuzz smoke (release branches and weekly CI):

```sh
sh scripts/fuzz_smoke.sh                # 5 000 libFuzzer runs per target
```

The conformance script writes `target/conformance-report.json`. A
non-zero exit code is a real failure — these scripts do not tolerate
divergence.

## CLI surface (stable)

```sh
target/release/nodx ast       doc.nodx
target/release/nodx validate  doc.nodx --format json
target/release/nodx html      doc.nodx > doc.html
target/release/nodx tui       doc.nodx
target/release/nodx ncp       doc.nodx
target/release/nodx semantic  doc.nodx
target/release/nodx integrity doc.nodx
target/release/nodx package   {inspect|verify} bundle.nodx
target/release/nodx export    {pdf|docx|pptx} doc.nodx -o out
target/release/nodx convert   {markdown-to-nodx|nodx-to-markdown} in out
```

Exit codes are normative: `0` success, `1` I/O/CLI usage, `2` parse or
validation or security failure, `3` unsupported required profile
(`NODX-E024`).

## Things to NOT do

- **Do not add `HashMap`/`HashSet` to the AST or canonical JSON path.**
  Use `BTreeMap<String, _>`. Iteration order is part of the contract.
- **Do not add a non-stdlib Rust dependency** without an explicit RFC
  discussion. Current externals are limited to `p256` and `serde_json`.
  `flate2`, `miniz`, `regex`, `serde_yaml`, `zip` are off-limits — there
  are hand-written replacements for the parts we need.
- **Do not write `unwrap()`/`panic!()`/`todo!()`/`unimplemented!()` on
  paths that consume untrusted input.** Use diagnostics. The handful of
  existing `unwrap()` calls are safe-by-construction and should be
  audited if you touch the surrounding code.
- **Do not duplicate URL or CSS safety logic.** Route through `nodx-url`
  and `nodx-style`. Adding a parallel check is how bypasses happen.
- **Do not edit `spec/conformance/v1.0/expected/` by hand.** Regenerate
  it with the reference implementation; see
  [`docs/reference/conformance.md`](./docs/reference/conformance.md).
- **Do not add raw HTML support, autolinks, setext headings, or
  CommonMark-only constructs** to the lite syntax. NODX is intentionally
  not CommonMark-conformant — see §28 of the RFC and the gap analysis
  notes. Markdown interop belongs in the `convert markdown-to-nodx`
  importer, not in the core grammar.
- **Do not introduce file writes** in parse/validate/render/projection/
  package-read code paths.
- **Do not silence a diagnostic.** If it is wrong, fix the document or
  raise the relevant cap (`ResourceLimits`). There is no
  `// nodx-ignore:` and there will not be one.
- **Do not commit secrets.** This is a public open-source project.
- **Do not skip hooks** (`--no-verify`) or signing flags. CI will reject
  the result.

## Glossary (terms that are easy to confuse)

| Term | Meaning |
|---|---|
| **Canonical AST** | Byte-stable JSON serialization of the document tree. The conformance contract. Sometimes called "Semantic AST" in older RFC sections — treat as synonyms. |
| **NCP** | NODX Compact Projection. Read-only semantic projection for agents/indexing. Schema `nodx-ncp/1.0`. |
| **Profile** | A capability set the reader must implement. `plain`, `core`, `rich`, `style`, `package`, `agent-read`, `remote-assets` (opt-in). |
| **Package** | A `.nodx` file whose first bytes are `PK\x03\x04`. Stored-ZIP only in baseline; DEFLATE is Rust-only. |
| **Fail-closed** | Unknown or unsafe input is rejected with a diagnostic, never silently accepted. |
| **Lite syntax** | The Markdown-shaped subset (headings, lists, tables, emphasis, code spans, links). Intentionally narrower than CommonMark. |
| **Full / node syntax** | The explicit `::name {attrs} ... ::` form for delimited blocks and custom components. |
| **Loss report** | JSON listing every NODX construct that did not survive a conversion (export PDF/DOCX/PPTX, NODX↔Markdown convert). |
| **Conformance bundle** | `spec/conformance/v1.0/` — fixtures + expected outputs. Versioned independently of the implementation. |

## Open known gaps (as of this writing)

These are not blockers but you should know about them before assuming
something works.

### Closed (do not re-introduce)

The following audit findings have been resolved; the listed safeguard
keeps them closed. If you find yourself working *against* one of them,
stop and discuss before proceeding.

- `NODX-E011`/`NODX-E015` are **reserved in baseline 1.0** (see RFC §23.1
  and [`docs/reference/diagnostics.md`](./docs/reference/diagnostics.md)).
  Guard: [`crates/nodx-validate/tests/reserved_diagnostic_codes.rs`](./crates/nodx-validate/tests/reserved_diagnostic_codes.rs)
  fails if any non-comment occurrence appears in the workspace.
- **Rust ↔ JS ↔ Python byte-for-byte parity** is now gated by
  [`scripts/run_conformance.sh`](./scripts/run_conformance.sh) (193
  fixtures × 4 outputs). The Python CLI driver lives at
  [`packages/nodx-py/bin/nodx-py.py`](./packages/nodx-py/bin/nodx-py.py).
- **JS/Py package readers** raise structured `NodxDiagnosticError`
  (`{code, severity, message}`) instead of language-native `Error`/
  `ValueError`. See [`packages/nodx-js/src/packageDiagnostics.mjs`](./packages/nodx-js/src/packageDiagnostics.mjs)
  and [`packages/nodx-py/src/nodx/package_diagnostics.py`](./packages/nodx-py/src/nodx/package_diagnostics.py).
  Severity table mirrors `crates/nodx-package/src/lib.rs::severity_for`.
- **Front-matter numeric canonicalization** in Python now collapses
  integer-valued floats (`1.0` → `1`) to match Rust's `f64::to_string()`
  and JS's `JSON.stringify(Number)`. See
  [`packages/nodx-py/src/nodx/front_matter.py`](./packages/nodx-py/src/nodx/front_matter.py)
  `_canonical_number` and `tests/test_front_matter_numbers.py`.
- **Text-side resource limits** (`sourceBytes`, `lineLength`,
  `frontMatterBytes`) are now enforced in JS/Py with `NODX-E012` fatal,
  matching `nodx_core::parse_str_with_limits`. See `blockParser.mjs` /
  `block_parser.py`.
- **Security corpus expectations** are machine-readable in
  [`spec/tests/security/{yaml-hostile,nods-hostile,xss}/expectations.json`](./spec/tests/security/)
  and pinned by `crates/nodx-validate/tests/security_corpus_expectations.rs`.
  A regression that silently accepts a hostile input flips that test red.
- **CLI integration tests** in [`crates/nodx-cli/tests/cli_smoke.rs`](./crates/nodx-cli/tests/cli_smoke.rs)
  pin exit codes (0/1/2/3), `NODX-E024` → exit 3, and the JSON shape of
  `diagnostics --format json`.
- `nodx-cli/src/main.rs` declares `#![forbid(unsafe_code)]` like every
  other crate entry point.
- **Thematic break (`hr`)** is recognized at the block level in Rust, JS,
  and Python parsers and renders as `<hr>` in HTML. The grammar is
  isolated-line `3*-` / `3*\*` / `3*_` with no internal whitespace (RFC
  §6.4). Front-matter disambiguation is structural (the opening `---` and
  its closer are consumed before block parsing). Fixture:
  [`spec/conformance/v1.0/fixtures/thematic-break.nodx`](./spec/conformance/v1.0/fixtures/thematic-break.nodx)
  plus the conformance triplet in
  [`spec/tests/conformance/thematic-break.nodx`](./spec/tests/conformance/thematic-break.nodx).
- **CommonMark-compatible inline + extended lists** (RFC §10.3, §12).
  Underscore emphasis (`_em_`, `__strong__`) follows the CommonMark
  intraword rule — alnum-flanked underscores stay literal so identifiers
  like `snake_case`, `__init__`, and `snake__case` are not mis-parsed.
  Code spans accept matching N-backtick runs (`` `single` ``,
  `` ``two ` ticks`` ``, `` ```three `` runs``` ``). Backslash escapes the
  extended set `` ` * [ ] ( ) { } # @ ~ ^ = : | _ ! . - + < > \ " ' ``,
  plus a trailing `\` before `\n` emits `Inline::LineBreak` (renders to
  `<br>`, projects to a space in plain text / NCP / Semantic Text). Lists
  accept `- `, `* `, `+ ` for unordered and `1.` / `1)` for ordered; the
  literal marker is *not* preserved in the AST. Triplet parity is locked
  by [`spec/conformance/v1.0/fixtures/inline-extensions.nodx`](./spec/conformance/v1.0/fixtures/inline-extensions.nodx)
  and [`spec/conformance/v1.0/fixtures/extended-lists.nodx`](./spec/conformance/v1.0/fixtures/extended-lists.nodx)
  plus the matching `spec/tests/conformance/*.nodx` siblings.
- **Autolinks `<scheme:...>` and `<email>` routed through `nodx-url`** (PR3,
  RFC §12). Both shapes compile to `Inline::Link` identical to the
  `[label](target)` form, so URL safety is gated exactly once via the
  Section 19 policy — no parallel allowlist in the parser. Bare emails
  acquire a `mailto:` prefix on the target so they are subject to the
  same whitelist. Disambiguation: any `<…>` with whitespace, an embedded
  `<`, a newline, or empty content stays literal text. Triplet parity is
  pinned by [`spec/conformance/v1.0/fixtures/autolinks.nodx`](./spec/conformance/v1.0/fixtures/autolinks.nodx)
  plus the matching `spec/tests/conformance/autolinks.nodx`; the unsafe
  case lives in [`spec/tests/negative/e020-autolink-javascript.nodx`](./spec/tests/negative/e020-autolink-javascript.nodx).
- **CommonMark-only constructs now surface as parser warnings**
  (`NODX-W030`..`NODX-W035`) instead of degrading silently. Emitters live
  in `crates/nodx-core/src/block_parser.rs` (`emit_block_commonmark_warnings`,
  `scan_inline_commonmark_warnings`) with byte-stable JS / Py twins in
  `packages/nodx-js/src/blockParser.mjs` and
  `packages/nodx-py/src/nodx/block_parser.py`. Codes are warning-severity,
  exit code stays `0`. Fixture: `spec/conformance/v1.0/fixtures/commonmark-warnings.nodx`.

### Still open

- **Stored-ZIP only in JS/Py package readers**; DEFLATE-compressed
  packages are Rust-only. The `examples/extended-showcase-bundled.nodx`
  fixture is intentionally excluded from triplet parity and exercised
  Rust-only at the end of `run_conformance.sh`.
- **Block-nesting / nodes-per-document caps** are enforced in Rust only.
  The text-side limits the JS/Py parsers now share are
  `sourceBytes`, `lineLength`, `frontMatterBytes`; the others remain
  best-effort outside Rust.
- **CommonMark interop** via `convert markdown-to-nodx` /
  `nodx-to-markdown` exists but is not a primary supported workflow.
  Several CommonMark constructs still degrade silently (no setext
  headings, no `> quote`, no inline `![img]`, no link reference
  definitions). See `convert` source and the gap analysis when
  extending. (Underscore emphasis, multi-backtick code spans, the
  thematic-break `***` form, hard line breaks, and `<scheme:...>` /
  `<email>` autolinks are now native — see the matching "Closed"
  entries.)

## How to update this file

When the architecture, invariants, or gates change, update this file in
the same PR. Keep entries one line where possible — the value of this
file is density, not completeness. The RFC is the long-form contract.

When you fix one of the "open known gaps" above, remove the entry. When
you discover a new one, add it. Treat this file as a working briefing,
not as documentation.

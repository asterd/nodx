# NODX — CommonMark interop alignment + new front-matter delimiter

You are working on the NODX project (a text-first, node-oriented document
format with byte-stable Canonical AST, fail-closed security, ZIP packaging,
and conformance profiles). Repository root: this working directory.

Authoritative documents (read them first, in order):

1. `AGENTS.md` — invariants, crate map, verification gates, things-to-NOT-do.
2. `NODX-RFC-0001.md` — normative spec. §6–§12 (syntax), §23.1 (diagnostics),
   §28 (interop), §30 (grammar summary).
3. `docs/reference/diagnostics.md` — current code registry.
4. `crates/nodx-core/src/block_parser.rs` / `inline_parser.rs` — current
   grammar implementation in Rust.
5. `packages/nodx-js/src/blockParser.mjs` / `inlineParser.mjs` — JS twin.
6. `packages/nodx-py/src/nodx/block_parser.py` / `inline_parser.py` — Py twin.
7. `scripts/run_conformance.sh` — Rust ↔ JS ↔ Py parity gate (must stay
   green at every milestone — 193 fixtures × 4 outputs byte-stable).

## Non-negotiable invariants (re-read AGENTS.md before touching code)

- Byte-stable Canonical AST. `BTreeMap<String, _>` everywhere in Rust;
  `Object.keys(...).sort()` in JS; `sorted(d)` in Py. No `HashMap` on the
  AST path.
- Fail-closed. All URL decisions go through `nodx-url`; all CSS through
  `nodx-style`; all package safety through `nodx-package`.
- `#![forbid(unsafe_code)]` on every Rust lib + the CLI main.
- No raw HTML, no script execution, no new external dependencies. The
  `Cargo.lock` external set today is `p256` + `serde_json` only.
- Rust ↔ JS ↔ Py parity is a release gate. **Run
  `sh scripts/run_conformance.sh` after every grammar change.**

## Verification gates (run at every milestone, all must be green)

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --release
node --test packages/nodx-js/test/*.mjs
python3 -m pytest packages/nodx-py/tests -q
sh scripts/run_conformance.sh
sh scripts/verify_conformance_package.sh

What this task is, in one paragraph
NODX 1.0 deliberately is not CommonMark-conformant; that stance is
correct and codified in §1 of the RFC. However, an audit identified
19 silent gaps where a CommonMark author writes valid Markdown and
NODX degrades it to paragraph text without emitting any diagnostic. The
purpose of this task is to (1) close the silent-gap problem with
warnings — never accept the construct, just tell the author it was
ignored — and (2) re-evaluate the front-matter delimiter (---) which
collides with the Markdown thematic break the warnings need to detect.

This is intentionally a two-phase task. Do not skip ahead. Each phase
ends in a green parity gate.

Phase A — Diagnose, do not transform
Add warning-level diagnostic codes that fire when the parser sees a
construct a CommonMark author would expect. The construct is still
treated as plain text (the AST does not change), but the warning makes
the silent degradation visible.

A.1 — Reserve diagnostic codes
In RFC §23.1 and docs/reference/diagnostics.md, reserve these new
codes as warning severity, owner core / markdown-detect:

Code	Trigger pattern	Hint
NODX-E030	Line matches ^(={3,}|-{3,})$ and previous line is non-blank text	Looks like a setext heading; use # H1 / ## H2.
NODX-E031	Line matches ^(\*\*\*|___)\s*$	Looks like a Markdown thematic break; not supported in NODX 1.0.
NODX-E032	Inline _text_ or __text__ with word boundaries	Underscore emphasis is not part of NODX lite syntax; use * / **.
NODX-E033	Line is `> ` content or blank `>`	Closed: native `::quote` aliases.
NODX-E034	Line starts with * or + (not nested)	Bullet markers other than - are not supported.
NODX-E035	Inline ![alt](url)	Markdown image is not supported inline; use :::image { src=... alt=... }.
NODX-E036	Inline <https://...> or autolink-shaped text	Autolinks are not supported; use [text](url).
NODX-E037	Inline [txt][ref] or [txt][]	Reference-style links are not supported in NODX 1.0.
NODX-E038	Line starts with ```	Backtick fenced code blocks are not supported; use ::code {lang="..."} ... ::.
Severity warning ⇒ exit code 0 (per docs/reference/diagnostics.md
exit-code table). Document each code in docs/reference/diagnostics.md
in the same PR.

A.2 — Emit the codes in the parser
Add detection to crates/nodx-core/src/block_parser.rs (for
E030/E031/E033/E034/E038) and crates/nodx-core/src/inline_parser.rs
(for E032/E035/E036/E037). Mirror the change in
packages/nodx-js/src/blockParser.mjs + inlineParser.mjs and
packages/nodx-py/src/nodx/block_parser.py + inline_parser.py.

The AST must not change. The line still becomes the same paragraph
or text node it does today; only doc.diagnostics grows.

A.3 — Add fixtures + gate
For each new code, add a negative fixture under spec/tests/negative/
named e030-*.nodx … e038-*.nodx. The existing test
negative_corpus_emits_expected_code_class
(crates/nodx-validate/src/lib.rs ~line 964) will pick them up
automatically if the file name follows the existing e0NN-*.nodx
convention.

A.4 — Update gap-analysis docs
Replace the "silent" bullet in AGENTS.md "Open known gaps →
CommonMark interop" with a one-liner: "NODX warns when it detects
Markdown-only constructs; see NODX-E030..E038."

A.5 — Parity gate
Run all 7 verification gates. If run_conformance.sh fails, the most
likely cause is that JS/Py emit the new codes in a different order than
Rust — fix the implementation, not the test.

Phase A is done when: all 7 gates are green and at least 9 new
fixtures (one per code) live under spec/tests/negative/.

Phase B — Re-evaluate the front-matter delimiter
The current --- delimiter for front matter collides with:

Markdown setext H2 underline (--- after a non-blank line).
Markdown thematic break.
YAML document separator (which NODX rejects anyway, but the visual ambiguity remains).
The collision matters because Phase A added NODX-E030/NODX-E031 that
also detect ----shaped lines. The parser disambiguates by position
(only --- at byte 0 after start or before EOF closes front matter),
but the rule is fragile and confusing to readers.

B.1 — Design alternatives (decide, do not implement)
Before touching code, write a one-page comparison in
docs/internals/front-matter-delimiter.md covering at least:

Keep --- (status quo).
+++ (Hugo/Zola convention — TOML/YAML signal).
---nodx / ---/nodx (typed delimiter; still YAML-looking).
:::front ... ::: (full node syntax; consistent with NODX's own block delimiters).
::: meta ... ::: (typed, namespace-prefixed).
For each option list: discoverability, collision risk with CommonMark
constructs E030/E031/E034, migration cost (existing *.nodx files in
this repo plus the conformance bundle), and back-compat ("can we accept
both --- and the new form during a transition window?").

Recommend the option you think is best, with one paragraph of
reasoning. Do not code anything yet.

B.2 — Hand the recommendation back
End Phase B with the design doc committed and a PR comment that asks
the maintainers for explicit go-ahead before implementation. The
parser/canonical AST/conformance bundle all depend on this delimiter;
changing it is a 1.x breaking change unless we accept both forms.

If approval is granted, the implementation work is roughly:

Lexer change in block_parser.{rs,mjs,py} to recognize the new opener and matching closer.
Update every .nodx fixture in examples/, spec/conformance/, spec/tests/, docs/ to use the new form (script-assisted: a one- shot Python script that rewrites ^---$ … ^---$ → new form, with review of the diff).
Add a deprecation warning NODX-E039 "legacy --- front-matter delimiter — migrate to <new>" that fires only when --- is seen at line 1, severity warning.
Update RFC §6.1 and §30 grammar summary.
Add fixtures that exercise both forms (until the deprecation window closes).
Run all 7 gates.
Constraints / things to NOT do (re-read AGENTS.md if unsure)
Do not change the AST shape during Phase A. Detection is side-channel; the canonical AST stays byte-identical.
Do not add any new external dependency.
Do not propose making NODX accept raw HTML, autolinks, setext headings, indented code, or any of the gap-1..19 constructs as behavior. They remain rejected/degraded; we only add visibility.
Do not edit spec/conformance/v1.0/expected/ by hand.
Do not skip the Python parity branch of run_conformance.sh to "make the demo green".
Do not amend or force-push.
Exit criteria
Phase A: 9 new diagnostic codes registered, emitted from all three parsers, covered by negative fixtures, gates green.
Phase B: design doc committed; no code change without explicit maintainer approval.
When in doubt, prefer the smaller change. The repo culture (and the
Karpathy guidelines surfaced in this session) prizes surgical edits and
verifiable goals.

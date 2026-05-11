# NODX 1.0 Conformance Report

**Date:** 11 May 2026  
**Scope:** Wave 07 release gate for the current reference implementation.

## Frozen Contracts

The following contracts are treated as frozen for the NODX 1.0 release line:

- Plain, Core, and Rich source syntax in `NODX_1.0_Working_Draft.md`.
- Canonical Semantic AST JSON shape and canonical JSON serialization rules.
- Semantic NCP projection shape used by the reference Rust and JavaScript
  implementations.
- Error registry codes and default severities in the working draft.
- CLI exit semantics documented in the working draft and implemented by
  `nodx-validate::exit_code_for`.
- Profile declaration field names and fail-closed unsupported required profile
  behavior.

## Corpus

The conformance runner processes the committed text fixture corpus and examples:

- `spec/tests/conformance/*.nodx`
- `spec/tests/ncp/*.nodx`
- `spec/tests/navigation/*.nodx`
- `spec/tests/rendering/*.nodx`
- `examples/*.nodx`
- `examples/i18n/*.nodx`
- `examples/print/*.nodx`

Packaged `.nodx` examples are inspected separately because JavaScript package
parity is not part of the 1.0 reader baseline.

Negative and security fixtures live under:

- `spec/tests/negative`
- `spec/tests/golden`
- `spec/tests/security`

## Required Gate

Run:

```sh
rtk cargo test
rtk sh scripts/run_conformance.sh
rtk git diff --check
```

The conformance script builds the Rust CLI, builds the packaged example,
compares Rust and JavaScript canonical AST output byte-for-byte, compares Rust
and JavaScript semantic NCP output byte-for-byte, renders HTML and TUI output
for every text fixture, verifies package inspection for the bundled example,
and writes `target/conformance-report.json`.

## Current Result Format

`target/conformance-report.json` contains one record per processed fixture:

```json
{"fixtures":[{"file":"spec/tests/conformance/core.nodx","ast":"ok","ncp":"ok"}]}
```

`ast` and `ncp` are `ok` for byte-identical Rust/JS output and
`skipped-packaged` for packaged examples outside the JS parity scope.

## Definition of Done Matrix

| Item | Status | Evidence |
|---|---|---|
| Plain, Core, Rich syntax frozen | done | `NODX_1.0_Working_Draft.md` |
| Canonical Semantic AST frozen | done | working draft, `crates/nodx-core/src/canonical.rs` |
| Error registry frozen | done | working draft Section 14 |
| CLI exit codes frozen | done | working draft, `crates/nodx-validate/src/lib.rs` |
| Profile declarations implemented | done | validator tests and negative fixtures |
| Rust parser passes corpus | verified by gate | `rtk sh scripts/run_conformance.sh` |
| JS parser passes corpus | verified by gate | `rtk sh scripts/run_conformance.sh` |
| Rust/JS AST parity | verified by gate | byte comparison in conformance script |
| Rust/JS NCP parity | verified by gate | byte comparison in conformance script |
| Validation-code fixtures | partial | committed negative/golden corpus covers implemented 1.0 diagnostics |
| Required unsupported features emit `NODX-E024` | done | `e024-unsupported-required-profile` fixture |
| CLI exits `3` for unsupported required features | done | validator unit test and CLI exit mapping |
| URL security corpus | partial | `spec/tests/security/url-policy.tsv` |
| YAML security corpus | partial | `spec/tests/security/yaml-hostile` |
| ZIP security corpus | partial | package tests plus `package-corpus.md` |
| NODS forbidden corpus | partial | `spec/tests/security/nods-hostile` |
| HTML/XSS corpus | partial | `spec/tests/security/xss` and renderer tests |
| Manifest digest verification | done | package reader tests |
| Package read never extracts | done | package reader exposes read-only in-memory `PackageFs` |
| Shared resource limits | done | `nodx-url::ResourceLimits` |
| Context escaping in HTML renderer | done | renderer tests and conformance render pass |
| Deterministic `toc` in HTML/NCP | done | navigation and NCP fixtures |
| No network sockets in reference crates | done | no socket APIs are used by reference crates |
| Fuzz targets exist | done | `fuzz/` |
| Release fuzz budget complete | not complete | see `fuzz/README.md` |
| Required release docs published | done | this document and companion docs |
| Known limitations explicit | done | `RELEASE_NOTES-1.0.md` |

## Known Conformance Limitations

The committed corpus is the full current fixture corpus, but it does not yet
meet the larger numeric corpus targets listed in
`NODX_1.0_Evolution_Plan.md` Section 12.2. The release notes track this as a
known limitation until CI or a release branch records the full target counts and
fuzz budget.

# NODX Public Test Suite

This directory contains public conformance fixtures for the NODX 1.0 reference
implementation.

Current invariant:

```sh
rtk sh scripts/run_conformance.sh
```

For every positive text `.nodx` fixture and example, the Rust reference parser
and the independent JavaScript parser must emit byte-identical canonical
Semantic AST JSON, semantic NCP JSON, and semantic text.

Negative fixtures pair with exact diagnostic JSON goldens under
`spec/tests/golden`. Security-focused inputs live under `spec/tests/security`.
Signature profile fixture documents live under `spec/tests/signature` and are
used by `crates/nodx-sign` tests for positive and tamper verification.
Presentation and exporter fixtures live under `spec/tests/presentation` and
`spec/tests/export`; they are consumed by `crates/nodx-export` tests and are
kept out of canonical conformance unless Rust/JS AST and NCP parity is required
for that fixture.
Historical release-gate notes are archived under `docs/archive/`.

Add new fixtures when adding syntax. Prefer small documents that isolate one behavior, plus one realistic mixed document.

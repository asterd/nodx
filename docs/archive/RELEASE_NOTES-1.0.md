# NODX 1.0 Release Notes

**Date:** 11 May 2026  
**Status:** 1.0 release-gate documentation for the current reference
implementation.

## Highlights

- Freezes the NODX 1.0 working draft as the source contract.
- Freezes canonical Semantic AST behavior and canonical JSON serialization.
- Freezes the error registry and CLI exit code semantics.
- Documents Rust/JavaScript AST and semantic NCP parity as the conformance
  gate.
- Publishes security, threat model, interoperability, conformance, migration,
  and fuzzing documentation.
- Adds fuzz target entry points for parser, inline parser, attributes, front
  matter, URL policy, package reader, NODS style validation, NCP serialization,
  and navigation resolution.
- Updates semantic NCP output schema from `nodx-ncp/0.1` to the frozen
  `nodx-ncp/1.0` contract.

## Known Limitations

- The committed fixture corpus is the full current corpus, but it does not yet
  meet every numeric corpus target from `NODX_1.0_Evolution_Plan.md` Section
  12.2.
- Release-candidate fuzzing has target entry points and documented commands,
  but the 24 CPU-hour per-target budget has not been completed locally.
- The safe NODS subset is implemented; the full style cascade and computed
  style model are deferred.
- The Rust package reader supports stored ZIP packages only. Deflated entries,
  signatures, package mutation, and trust policy are deferred.
- Signature, editor/lossless CST, presentation, and agent mutation profiles are
  deferred future profiles.
- The workspace does not yet contain a separate `nodx-ncp` crate; semantic NCP
  lives in `nodx-core` and JavaScript parity code.
- Native PDF, DOCX, and PPTX exporters are not part of NODX 1.0.
- Media type registration has a documented plan, but IANA registration is not
  complete.

## Compatibility

Documents should use:

```yaml
schema: nodx/1.0
profiles:
  requires: [core]
```

Unsupported required profiles fail closed with `NODX-E024`; the CLI exits with
code `3`. Unsupported optional profiles warn with `NODX-E023`.

## Verification

Release verification commands:

```sh
rtk cargo test
rtk sh scripts/run_conformance.sh
rtk git diff --check
```

Fuzzing commands are documented in `fuzz/README.md`.

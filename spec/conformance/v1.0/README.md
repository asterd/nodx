# NODX 1.0 Conformance Package

This package is the small, versioned implementer-facing conformance set. It is
not a replacement for the larger repository corpus under `spec/tests`; it is a
stable starter set for external parser, validator, renderer, and NCP
implementations.

## Contents

- `fixtures/`: source `.nodx` inputs.
- `expected/`: canonical outputs produced by the Rust reference CLI.
- `manifest.json`: machine-readable fixture index.

## Verification

```sh
rtk sh scripts/verify_conformance_package.sh
```

An implementation should match `*.ast.json`, `*.ncp.json`, and diagnostic JSON
byte-for-byte. HTML is included as reference output for the bundled renderer;
other renderers may differ visually, but must preserve safety and semantics.

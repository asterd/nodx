# NODX 1.0 Conformance Package

This package is the small, versioned implementer-facing conformance set. It is
not a replacement for the larger repository corpus under `spec/tests`; it is a
stable starter set for external parser, validator, renderer, and NCP
implementations.

It includes both classic 1.0 syntax and NODX-Lite authoring forms: two-colon
blocks, heading light IDs, short variables, link attributes, manual TOC, and
Markdown-compatible table separators.

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

# NODX Public Test Suite

This directory contains public conformance fixtures for NODX 0.1 and the
targeted NODX 1.0 milestone corpora.

Current invariant:

```sh
rtk sh scripts/run_conformance.sh
```

For every positive `.nodx` fixture and example, the Rust reference parser and
the independent JavaScript parser must emit byte-identical canonical Semantic
AST JSON. NCP fixtures also require byte-identical Rust and JavaScript semantic
NCP JSON.

Add new fixtures when adding syntax. Prefer small documents that isolate one behavior, plus one realistic mixed document.

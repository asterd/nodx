# NODX Public Test Suite

This directory contains public conformance fixtures for NODX 0.1.

Current invariant:

```sh
sh scripts/run_conformance.sh
```

For every `.nodx` fixture and example, the Rust reference parser and the independent JavaScript parser must emit byte-identical canonical Semantic AST JSON.

Add new fixtures when adding syntax. Prefer small documents that isolate one behavior, plus one realistic mixed document.


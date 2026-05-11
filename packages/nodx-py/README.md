# nodx Python

Python implementation of the NODX parser and deterministic projections.

The public API mirrors the JavaScript package where Python naming conventions
allow it:

```python
from nodx import canonical_json, ncp_json, parse, validate

doc = parse("# Hello #hello\n")
print(canonical_json(doc))
print(ncp_json(doc))
print(validate(doc))
```

Implemented capability surface:

- text parser, inline parser, canonical AST JSON, diagnostics, and NCP;
- URL/package path policy;
- stored ZIP package reader compatible with the JavaScript reader;
- HTML fragment/document renderer, standard theme stylesheets, and semantic text;
- NODS audit, sanitization, and YAML style conversion.

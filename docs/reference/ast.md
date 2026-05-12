# Canonical AST

The Semantic AST is the single source of truth for a NODX document. Every
operation — rendering, validation, packaging, signing, agent extraction —
operates on it, not on the source text.

## Why canonical

"Canonical" means *byte-stable*. Two conformant parsers fed the same input
must produce the same bytes of JSON. We get there by:

- Sorting all object keys.
- Using a fixed numeric encoding (no `1.0` vs `1`; integers stay integers).
- Encoding strings via a fixed escape set.
- Emitting attribute lists in lexicographic order.
- Lowering syntactic sugar (lite vs full block form, heading id sugar,
  variable namespacing) into a single AST representation.

This is what makes `git diff` on a canonical AST meaningful, what makes
content hashing reliable, and what makes the conformance suite a real test
instead of a vibe check.

## Document shape

```json
{
  "schema": "nodx/1.0",
  "type": "document",
  "meta": { /* canonical front matter */ },
  "body": [ /* array of nodes */ ],
  "diagnostics": [ /* in source order */ ]
}
```

`meta` always contains at least `schema`, `type`, `language`, `dir`, and
`profiles`. The parser fills in defaults when the document omits them.

## Node shape

```json
{
  "type": "note",
  "id": "safe-note",
  "classes": ["callout"],
  "attrs": {"type": "warning"},
  "styles": {},
  "inlines": [ /* inline children */ ],
  "children": [ /* nested blocks */ ],
  "text": null
}
```

| Field | Type | Meaning |
|---|---|---|
| `type` | string | One of the block names (`heading`, `paragraph`, `note`, …) or a custom component name. |
| `id` | string \| null | Stable id. Either authored (`#intro`) or absent. |
| `classes` | string[] | Sorted, deduplicated. |
| `attrs` | object | Typed attribute map. Keys sorted. |
| `styles` | object | Safe CSS shorthand expansions. Keys sorted. |
| `inlines` | inline[] | For *textual* nodes (heading, paragraph, item, cell). |
| `children` | node[] | For *container* nodes (section, note, table, list, …). |
| `text` | string \| null | For *literal* nodes (code, pre, math, style); raw text preserved verbatim. |

A node is either textual *or* container *or* literal. The other two arrays
are empty (or `text` is `null`).

## Inline shape

Inlines are a tagged union:

```json
{"type": "text", "value": "Hello"}
{"type": "strong", "children": [{"type": "text", "value": "loud"}]}
{"type": "em", "children": [...]}
{"type": "mark", "children": [...]}
{"type": "sub", "children": [...]}
{"type": "sup", "children": [...]}
{"type": "code", "value": "x + 1"}
{"type": "math-inline", "source": "x^2"}
{"type": "link", "target": "https://example.com", "label": [...], "attrs": {...}}
{"type": "span", "children": [...], "attrs": {...}}
{"type": "var", "namespace": "vars", "name": "reviewer"}
{"type": "ref", "target": "intro"}
{"type": "footnote-ref", "target": "fn1"}
{"type": "citation-ref", "target": "smith2024"}
{"type": "mention", "kind": "user", "target": "alice"}
```

Variables are always namespaced in the canonical AST: `{{name}}` in source
becomes `{"namespace": "vars", "name": "name"}` in the AST.

## Where the canonicalization happens

Source code: [`crates/nodx-core/src/canonical.rs`](../../crates/nodx-core/src/canonical.rs).

The CLI exposes it as `nodx ast`:

```sh
nodx ast doc.nodx > doc.ast.json
```

Two ways to verify byte-stability:

1. Round-trip: `nodx ast doc.nodx | sha256sum` twice; the digest is the
   same.
2. Cross-implementation: `sh scripts/run_conformance.sh` compares the Rust
   and JavaScript reference parsers byte for byte across every committed
   fixture.

## Things the AST intentionally does not preserve

- Source whitespace (except inside literal blocks, where it is part of the
  payload).
- Comment markers — NODX does not have a comment syntax.
- The exact spelling of attribute syntax (`{.cls}` vs `class="cls"`).
- The Lite vs full block form. Both collapse to the same node.

If you need to round-trip the original source text — for example, in an
editor — use the *CST* (concrete syntax tree) from `nodx-cst` instead.
The CST keeps byte ranges and is what powers the editor contract.

# NCP — NODX Content Projection

NCP is the agent-readable view of a NODX document. It is a JSON projection
of the canonical AST, designed for retrieval pipelines, embeddings, and
batch agent workflows that need a stable, semantic-aware chunking.

## Generating NCP

```sh
nodx ncp doc.nodx              # summary mode (default)
nodx ncp doc.nodx --mode chunks
nodx ncp doc.nodx --mode full
```

The output is JSON, byte-stable across runs for the same input.

## The three modes

| Mode | Adds | Best for |
|---|---|---|
| `summary` | Per-node id, kind, path, hash, headings, first ~120 chars of text. | Index population, search hit highlighting. |
| `chunks` | Same as `summary` plus the full plain text body of each node. | Embeddings, retrieval-augmented generation. |
| `full` | Same as `chunks` plus inline annotations (references, citations, mentions) and resolved cross-links. | Agent reasoning over the structured document. |

A consumer that needs the smallest stable view should always use `summary`.
Adding modes is monotonic — `chunks` is a superset of `summary`,
`full` is a superset of `chunks`.

## Top-level shape

```json
{
  "schema": "ncp/1.0",
  "document": {
    "id": "doc-hash",
    "title": "…",
    "language": "en",
    "profile": ["core", "rich"],
    "hash": "sha256-…"
  },
  "nodes": [ /* one entry per AST node */ ],
  "links": [ /* resolved internal references */ ]
}
```

## Per-node shape

```json
{
  "id": "intro",
  "path": [0],
  "kind": "section",
  "level": 1,
  "headings": ["Introduction"],
  "text": "First sentence of the section…",
  "hash": "sha256-…",
  "attrs": {"type": "info"}
}
```

| Field | Meaning |
|---|---|
| `id` | The document-stable id if the node has one; otherwise the auto-derived `path`-id. |
| `path` | The index path from `body[]` to the node. Same as the validator's source location. |
| `kind` | `heading`, `paragraph`, `note`, `section`, …. |
| `level` | Heading depth, or `null` for non-heading nodes. |
| `headings` | The chain of ancestor headings leading to this node. Useful for displaying retrieval context. |
| `text` | Plain text of the node (and, in `chunks`/`full` modes, of its descendants). |
| `hash` | SHA-256 of the canonical AST subtree, base64url-encoded. Used for change tracking. |
| `attrs` | Block attributes that survived validation. |

## Why hashes

Every node in the AST has a stable content hash. NCP exposes them at the
top level so an agent can:

- Detect that "the same section, but rephrased" produced a different hash
  and re-embed only that node.
- Skip nodes whose hash matches a cached embedding.
- Verify that a write back from an agent (`agent-sdk` `Operation`)
  applies cleanly: the operation carries the *expected* hash, and is
  rejected if the live document has drifted.

The hash function is documented in
[`crates/nodx-core/src/hashing.rs`](../../crates/nodx-core/src/hashing.rs);
the agent write API is in
[`crates/nodx-agent-sdk`](../../crates/nodx-agent-sdk).

## Why path

NCP encodes structure in two ways: hierarchical (`headings[]`) and
positional (`path[]`). A retrieval system can pick whichever is more
useful for its UI. Stable ids exist for the case where neither will do.

## Inline projection (full mode only)

```json
{
  "kind": "paragraph",
  "text": "See @[intro] and footnote [^fn-1].",
  "annotations": [
    {"kind": "ref", "target": "intro", "start": 4, "end": 11},
    {"kind": "footnote-ref", "target": "fn-1", "start": 27, "end": 34}
  ]
}
```

Annotations carry character offsets into the plain text body. They are
the right primitive to build "hover to preview the referenced section"
UIs and to walk a citation graph across documents.

## When *not* to use NCP

NCP is not a rendering format. It does not preserve styling, theme
attributes, or fallback content for unknown components. Use the canonical
AST (`nodx ast`) when you need full fidelity, the HTML output (`nodx
html`) when you need presentation.

Use NCP when:

- You are building an index over many documents.
- You are feeding documents to an LLM that benefits from semantic chunks.
- You are diffing a document at the *semantic* layer (across rewordings).

Don't reach for it when "the document, exactly as it is" is what you
actually need.

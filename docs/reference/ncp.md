# NCP And Semantic Text

NODX exposes two standard agent-readable projections:

- **NCP**: deterministic JSON for tools that need node addresses, hashes,
  attributes, children, and resolved navigation.
- **Semantic text**: compact UTF-8 text for LLM context, search snippets, and
  quick human review.

Use the Canonical AST when you need full fidelity. Use rendered HTML when you
need presentation.

## Generating Projections

```sh
nodx ncp doc.nodx
nodx ncp doc.nodx --mode semantic
nodx semantic doc.nodx
```

`nodx ncp` emits `application/nodx-ncp+json`. `nodx semantic` emits
`text/nodx-semantic; charset=utf-8`.

## NCP Shape

NCP 1.0 uses schema `nodx-ncp/1.0` and mode `semantic`:

```json
{
  "chunks": [
    {
      "id": "chunk-1",
      "nodes": ["intro", "path:1"],
      "sha256": "sha256-..."
    }
  ],
  "loss": [],
  "mode": "semantic",
  "nodes": [
    {
      "attrs": {"level": "1"},
      "children": [],
      "id": "intro",
      "path": "0",
      "sha256": "sha256-...",
      "text": "Introduction",
      "type": "heading"
    }
  ],
  "schema": "nodx-ncp/1.0",
  "sourceHash": "sha256-..."
}
```

`nodes` is a tree projection: the top-level array mirrors `body[]`, and every
record carries recursive `children`. `path` is a dot-separated structural path
from `body[]`. `id` is the source ID or an empty string. Chunk node references
use source IDs when present and `path:<path>` otherwise.

For `toc` nodes, NCP adds `navigationEntries` with resolved `id`, `level`,
`path`, and `title` fields.

## NCP Exclusions

NCP is not a Canonical AST clone. It deliberately excludes concrete syntax
trivia, computed CSS, renderer templates, host layout results, package manifest
metadata, and rendered custom component HTML. `loss: []` means no loss inside
the NCP semantic contract, not that every Canonical AST field is present.

Custom components remain ordinary node records. Consumers that do not
understand a custom component should read its children as fallback source
content.

## Semantic Text

Semantic text is a compact, deterministic text projection:

```text
# Introduction #intro
Paragraph text.

Component approval-card [id="approval" status="pending" fallback="children"]:
Fallback content.
```

It includes readable source content: headings, paragraphs, lists, tables,
figures, images, captions, literal code/math, quotes, notes, forms, media
fallbacks, bibliography entries, and custom component fallback children.

It excludes style nodes, component styles, `toc`, `pagebreak`, automatic page
boundaries, renderer-generated HTML, computed CSS, package metadata, and custom
component template output. Output always ends with one trailing newline.

Use semantic text when token cost matters and the consumer does not need stable
node hashes or patch addresses. Use NCP for agent tools that need to cite,
cache, diff, or update specific nodes.

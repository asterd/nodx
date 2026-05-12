# Implementer guide

> This page used to be the single landing point for implementers. The
> material has moved into dedicated reference pages so each topic can
> grow without becoming a wall of text. The pointers below are the new
> canonical locations.

## Where things live now

| If you want to… | Read |
|---|---|
| Understand the wire contract | [NODX-RFC-0001](../NODX-RFC-0001.md) |
| See the AST shape every implementation must produce | [reference/ast.md](./reference/ast.md) |
| Look up a diagnostic code | [reference/diagnostics.md](./reference/diagnostics.md) |
| Check which profiles exist | [reference/profiles.md](./reference/profiles.md) |
| Run the cross-implementation tests | [reference/conformance.md](./reference/conformance.md) |
| Inspect resource limits | [reference/limits.md](./reference/limits.md) |
| Walk the reference crate graph | [internals/architecture.md](./internals/architecture.md) |
| Understand the security boundary | [internals/security-model.md](./internals/security-model.md) |

## Minimal reader checklist

A reader is *useful* when it implements:

- paragraphs and headings;
- front matter under `schema: nodx/1.0`;
- delimited blocks with fallback children, in both the two-colon Lite
  form and the three-colon full form;
- safe inline parsing: text, code, link with `{attrs}`, `{{name}}` and
  `{{namespace.name}}` variables;
- pipe tables and lists, including task lists;
- diagnostics JSON;
- `NODX-E024` for unsupported required profiles, with exit code `3`.

A reader is *conformant* when it produces byte-identical canonical AST,
NCP, and diagnostics output against the conformance package.

## Media types

| Format | Media type | Alias |
|---|---|---|
| Text NODX | `text/nodx; charset=utf-8` | `text/x-nodx` |
| Packaged NODX | `application/nodx+zip` | `application/x-nodx+zip` |

The aliases are tolerated until media-type registration is complete.

## Editor integrations

For editor extensions, do not implement a partial validator. Shell out
to `nodx validate --format json` for diagnostics, `nodx html` for
preview, and `nodx ncp` for outline. Partial validators silently drift
from the spec and are the largest single source of conformance reports.

If you do want to vendor a parser, vendor `packages/nodx-js` directly
and run the conformance suite against it on every release.

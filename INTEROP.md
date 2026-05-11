# NODX 1.0 Interoperability Notes

## Reference Implementations

NODX 1.0 conformance is anchored by two independent text implementations:

- Rust crates under `crates/`, exposed through the `nodx` CLI.
- JavaScript parser and NCP projector under `packages/nodx-js`.

The release gate requires byte-identical canonical Semantic AST and semantic
NCP output for every committed text fixture and example included by
`scripts/run_conformance.sh`.

## Canonical Semantic AST

Interoperable processors must preserve:

- node ordering;
- node `type`, `id`, `classes`, `attrs`, `children`, `inlines`, and `text`;
- metadata values from the safe front matter subset;
- deterministic sorted object keys in canonical JSON;
- UTF-8 source handling and forbidden byte diagnostics.

Processors must not include diagnostics, source ranges, CST trivia, renderer
state, computed style, or editor state in canonical Semantic AST output.

## Semantic NCP

Semantic NCP output is intended for read-only agent and retrieval consumption.
It must be deterministic for the same canonical AST. Implementations must keep
node paths, node hashes, document source hash, and resolved `toc` entries stable
unless the canonical AST or NCP contract changes in a future major version.

## Profile Interop

Supported NODX 1.0 profile short names are:

- `plain`
- `core`
- `rich`
- `style`
- `package`
- `agent-read`

Reserved future profiles such as `agent-mutate`, `signature`, `editor`, and
`presentation` must fail closed when required and warn when optional.

## Packages

The Rust reference reader supports safe stored-ZIP packages with manifest and
digest verification. Interop-sensitive package behavior:

- first entry is `mimetype`;
- `mimetype` bytes are `application/nodx+zip`;
- `manifest.yaml` is required;
- manifest-listed assets are checked for size and SHA-256 digest when present;
- package paths are normalized and cannot escape the package root;
- entries are read into an in-memory read-only filesystem and are never
  extracted to disk.

Deflated entries, signatures, package mutation, and advanced package trust
policy are future work.

## Media Type Registration Plan

The target registered media types are:

- `text/nodx; charset=utf-8` for Text NODX.
- `application/nodx+zip` for Packaged NODX.

Until registration is complete, integrations should use experimental names:

- `text/x-nodx`
- `application/x-nodx+zip`

Planned registration steps:

1. Prepare IANA media type templates from the frozen working draft.
2. Include security considerations from `SECURITY.md` and `THREAT_MODEL.md`.
3. Reference the file extension `.nodx` and byte-sniffing distinction between
   UTF-8 text and ZIP packages.
4. Submit `text/nodx` and `application/nodx+zip` together so processors can map
   the two representations consistently.
5. Keep experimental media types documented until IANA review is complete.

# NODX 1.0 Implementer Guide

This guide is non-normative. The normative contract is `NODX-RFC-0001.md`.

## Implementation Order

1. Detect representation by bytes:
   - `PK\x03\x04` means packaged NODX.
   - Otherwise parse as UTF-8 text NODX.
2. Enforce resource limits before expensive work.
3. Validate UTF-8, BOM policy, and U+0000.
4. Parse front matter using the NODX YAML safe subset.
5. Parse blocks, headings, lists, tables, attributes, and inline syntax.
6. Produce canonical Semantic AST JSON with sorted object keys.
7. Run semantic validation and emit diagnostics.
8. Implement URL/path policy.
9. Implement NCP semantic projection.
10. Implement renderer output with context escaping.

## Minimal Reader

A minimal useful reader should support:

- paragraphs and headings;
- front matter `schema: nodx/1.0`;
- delimited blocks with fallback children;
- safe inline text/code/link parsing;
- diagnostics JSON;
- unsupported required profiles as `NODX-E024` and exit code `3`.

## Canonical Output

Canonical AST and NCP must be byte-stable. Use:

```sh
rtk sh scripts/verify_conformance_package.sh
```

Compare your implementation against `spec/conformance/v1.0/expected`.

## Diagnostics

Diagnostic JSON fields are fixed:

```json
{"code":"NODX-E024","severity":"error","message":"Required profile `x` is unsupported.","line":null,"column":null,"target":"profile:x"}
```

Warnings and info diagnostics must not cause non-zero exit codes. Fatal and
error diagnostics use exit code `2`, except unsupported required capabilities,
which use exit code `3`.

## Media Types

Use these names in integrations:

- Text NODX: `text/nodx; charset=utf-8`
- Packaged NODX: `application/nodx+zip`

Until registration is complete, tools may also accept `text/x-nodx` and
`application/x-nodx+zip` as aliases.

## Editor Integration Contract

An editor integration does not need to implement the full parser. A marketable
extension should provide:

- syntax highlighting for `.nodx`;
- snippets for front matter, `toc`, `section`, `note`, `figure`, and `table`;
- validation by invoking `nodx diagnostics --format json`;
- preview by invoking `nodx html`;
- optional outline from `nodx ncp`.

Do not implement a partial validator in the editor unless it is tested against
the conformance package.

# Migrating NODX 0.1 Documents to NODX 1.0

NODX 1.0 preserves the practical 0.1 text surface where possible, but freezes
previously draft behavior into explicit contracts. This guide lists breaking or
externally visible changes.

## Schema

Old:

```yaml
schema: nodx/0.1
```

New:

```yaml
schema: nodx/1.0
```

Expected diagnostic for missing or unsupported schema:

```text
NODX-E004 Missing or invalid schema for NODX.
```

Mechanical patch: update the `schema` field in front matter.

## Profiles

Old 0.1 documents often omitted capability declarations or used ad hoc
`requires` values.

New:

```yaml
profiles:
  requires: [core, rich]
  optional: [style]
```

Unsupported required profiles now produce `NODX-E024` and the CLI exits with
code `3`. Unsupported optional profiles produce `NODX-E023` and do not fail the
CLI unless other errors exist.

Mechanical patch: move feature requirements to `profiles.requires` and optional
capabilities to `profiles.optional`.

## Front Matter Safe Subset

Old:

```yaml
---
schema: nodx/0.1
defaults: &defaults
  title: Example
page: *defaults
---
```

New:

```yaml
---
schema: nodx/1.0
page:
  title: Example
---
```

Expected diagnostic:

```text
NODX-E019 Forbidden YAML construct.
```

Mechanical patch: expand anchors, aliases, merge keys, explicit tags, duplicate
keys, multiple documents, timestamps, binary tags, custom objects, and
non-finite numbers before importing the document.

## URLs and Asset References

Old:

```nodx
[local](file:///etc/passwd)
![diagram](../private.png)
```

New:

```nodx
[local](#safe-section)
![diagram](assets/diagram.png)
```

Expected diagnostics:

```text
NODX-E020 Unsafe URL or scheme.
NODX-E010 Unsafe path or path traversal.
```

Mechanical patch: use fragment links, allowed web link schemes, or
package-relative asset paths that do not contain absolute paths, backslashes,
empty path parts, or `..`.

## Packages

0.1 package examples were draft artifacts. 1.0 packages must use safe stored ZIP
entries and a manifest with verified entry metadata.

New package requirements:

- first ZIP entry is `mimetype`;
- `mimetype` content is `application/nodx+zip`;
- `manifest.yaml` exists and names the entry document;
- manifest paths pass package path validation;
- listed sizes and SHA-256 digests match the actual entry bytes.

Expected digest diagnostic:

```text
NODX-E021 Package digest mismatch.
```

Mechanical patch: rebuild packages with `scripts/build_package.py` or an
equivalent stored-ZIP writer that emits deterministic paths and manifest data.

## Style Blocks

Old style blocks could contain draft CSS-like constructs.

New 1.0 behavior accepts only the safe NODS subset and omits unsafe rules from
rendered HTML.

Expected diagnostic:

```text
NODX-E027 Forbidden NODS construct.
```

Mechanical patch: remove imports, executable content, forbidden selectors, and
unsafe `url(...)` references; keep declarations in the documented safe subset.

## Canonical AST and NCP

Canonical AST and NCP output are frozen at 1.0. NCP projections now identify
their schema as `nodx-ncp/1.0` instead of `nodx-ncp/0.1`.

Do not rely on 0.1 renderer bytes, draft field order, or non-semantic trivia.
Regenerate downstream hashes from the 1.0 canonical AST and semantic NCP output.

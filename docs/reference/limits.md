# Resource limits

NODX is designed to *fail closed* on hostile input. The parser, validator,
and package reader carry an explicit `ResourceLimits` value; every
expensive operation checks against it before doing work.

## The defaults

Source: [`crates/nodx-core/src/limits.rs`](../../crates/nodx-core/src/limits.rs).

| Field | Default | Enforced? | Where |
|---|---|---|---|
| `source_bytes` | 64 MiB | yes | parser, package reader |
| `front_matter_bytes` | 64 KiB | yes | parser |
| `line_length` | 1 MiB | yes | parser (warn) |
| `attribute_value_bytes` | 64 KiB | yes | parser (`parse_attrs`) |
| `id_bytes` | 256 | yes | validator |
| `block_nesting_depth` | 32 | yes | parser |
| `inline_nesting_depth` | 32 | **reserved** | future inline composer |
| `nodes_per_document` | 100 000 | yes | parser |
| `data_uri_bytes` | 5 MiB | yes | `nodx-url` |
| `expanded_ast_bytes` | 64 MiB | **reserved** | future expansion pass |
| `include_depth` | 8 | **reserved** | future `::include` extension |
| `package_uncompressed_bytes` | 256 MiB | yes | `nodx-package` |
| `package_file_count` | 1 024 | yes | `nodx-package` |
| `package_entry_bytes` | 64 MiB | yes | `nodx-package` |
| `package_compression_ratio` | 100× | yes | `nodx-package` |
| `package_nested_zip_depth` | 0 | **reserved** | reader currently rejects all nested zips outright |
| `package_path_bytes` | 512 | yes | `nodx-package` |
| `package_path_segments` | 8 | yes | `nodx-package` |
| `url_bytes` | 4 KiB | yes | `nodx-url` |
| `manifest_entries` | 1 024 | yes | `nodx-package` |
| `signature_header_bytes` | 8 KiB | yes | `nodx-sign` |
| `export_bytes` | 256 MiB | yes | `nodx-export` |
| `export_entry_count` | 1 024 | yes | `nodx-export` |

The "reserved" rows are the truthful part of this table: those fields are
kept on the struct for forward compatibility (so callers that already
pass them don't need to migrate when the corresponding feature lands), but
no current code path consumes them. The [scalability
notes](../internals/scalability.md) explain why.

## How a limit is enforced

When a limit is hit:

1. The component emits a `NODX-E012` diagnostic with a message that names
   the limit (`"Node count limit exceeded."`, `"Package entry size limit
   exceeded."`, …).
2. The component stops creating more of whatever was overflowing — more
   nodes, more depth, more bytes. It does not throw, it does not panic,
   and it does not corrupt the partial result.
3. The caller decides whether to continue. The CLI treats `NODX-E012` as
   fatal (exit code `2`).

This is a *cooperative* protection model: a malicious input is bounded,
but a well-meaning input that legitimately needs more headroom can be
handled by raising the relevant cap.

## How to raise a cap

Programmatic, in Rust:

```rust
use nodx_core::{parse_str_with_limits, ResourceLimits};

let mut limits = ResourceLimits::default();
limits.nodes_per_document = 500_000;
limits.source_bytes = 128 * 1024 * 1024;

let doc = parse_str_with_limits(&source, limits);
```

Programmatic, in JavaScript:

```js
import { parseString } from "nodx";
const doc = parseString(source, {
  limits: { nodesPerDocument: 500_000, sourceBytes: 128 * 1024 * 1024 },
});
```

The CLI does not expose `--limits-*` flags. It uses
`ResourceLimits::default()`. This is intentional: a one-shot CLI run is
not the place to relax safety bounds. If you need higher caps, write a
small driver program against the library.

## Choosing sensible numbers

Some heuristics that have been validated by benchmarks (see
[scalability](../internals/scalability.md)):

| Document size | Suggested caps |
|---|---|
| Small (a memo, a README) | defaults |
| Medium (a 200-page report) | defaults |
| Large (a 2 000-page book, rich content) | defaults — fits in 80 MB of RAM and parses in under a second |
| Very large (5 000+ pages) | raise `nodes_per_document` to 500 000, `source_bytes` to 128 MiB |
| Extreme (10 000+ pages) | raise `nodes_per_document` to 1 000 000, `source_bytes` to 256 MiB. Expect ~400 MB RAM peak. |

The product of `nodes_per_document` × ~1 KiB AST overhead per node is a
useful upper bound on RAM. The reference implementation does not stream
yet; everything fits in memory.

## What the limits are *not*

- **Not a sandbox.** NODX does not execute code at any point. The limits
  protect *the parser*, not the host process at large.
- **Not a quota system.** There is no token bucket, no per-tenant
  accounting, no time slicing. If you need that, run NODX inside a
  process boundary with cgroups or equivalent.
- **Not a versioning gate.** Limits are tuning knobs, not contract
  changes. Two readers with different caps still parse the same canonical
  AST when the document fits in both.

## When to override defaults in production

There are two legitimate reasons:

1. **You ship long documents.** Books, technical references, knowledge
   bases. Raise `nodes_per_document` and `source_bytes` together.
2. **You ship packages with many small assets.** Brand kits, design
   systems. Raise `package_file_count` and `package_uncompressed_bytes`
   together.

There is no legitimate reason to *lower* the caps below the defaults; the
defaults are already conservative enough for hostile-input scenarios. If
your environment requires harder bounds, lower them via the API, but
remember that a *useful* document and a hostile document can look the
same from the parser's perspective.

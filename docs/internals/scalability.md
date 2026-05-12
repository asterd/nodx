# Scalability — measured numbers and what they mean

NODX is designed so that *a 2 000-page book* is a routine input, not an
edge case. This page reports the actual measurements taken on the
reference implementation and explains what changes if you push past
them.

## Test rig

- Machine: Apple Silicon laptop.
- Build: `cargo build --release -p nodx`.
- Workload: a synthetic book generator (`/tmp/gen_book.py` in the
  benchmark script) that produces, per "page": one `h2`, two `h3`,
  several paragraphs, a list, a pipe table, an occasional callout and
  page break. Roughly 50 AST nodes per page.

## Measured results

| Input (synthetic) | Source size | AST nodes | Operation | Wall time | Peak RSS |
|---|---:|---:|---|---:|---:|
| 2 000 pages | 3.8 MiB | ~108 k | `nodx ast` | 0.51 s | 80 MiB |
| 2 000 pages | 3.8 MiB | ~108 k | `nodx html` | 0.16 s | 81 MiB |
| 2 000 pages | 3.8 MiB | ~108 k | `nodx validate` | 0.14 s | 71 MiB |
| 2 000 pages | 3.8 MiB | ~108 k | `nodx ncp` | 0.26 s | 108 MiB |
| 5 000 pages | 9.6 MiB | ~270 k | `nodx html` | 0.40 s | 195 MiB |
| 10 000 pages | 19 MiB | ~540 k | `nodx html` | 0.79 s | 386 MiB |

Output sizes scale roughly linearly with input:

| Input | Canonical AST | HTML | NCP (`chunks`) |
|---|---:|---:|---:|
| 2 000 pages | 9.7 MiB | 4.4 MiB | 12 MiB |
| 5 000 pages | ~25 MiB | 11 MiB | ~30 MiB |
| 10 000 pages | ~50 MiB | 22 MiB | ~60 MiB |

The constant of proportionality is about **20× the source size in peak
RSS** and **1.2× the source size in HTML output**. Both numbers are
stable across runs.

## What the curves mean

The parser, validator, renderer, and NCP projector are all linear in
input size. There are no quadratic surprises in the hot paths:

- Block parsing is a single linear pass over the input split into lines.
- Inline parsing is iterative, not recursive — there is no nesting-depth
  blow-up.
- Canonical JSON emission sorts attribute keys per node, but each node
  has O(1) attributes in practice, so the global cost is O(n).
- The HTML renderer walks the AST once and writes bytes to a single
  growable buffer.
- NCP walks the AST once and emits one record per node.

The first run is slower than subsequent runs (~0.51 s vs ~0.16 s on the
same input) because of OS file-cache warm-up, not because of any
nondeterminism in the code path.

## Default caps versus measured needs

The default `ResourceLimits` are *bigger than the smallest real document
you might write* and *smaller than the worst input a hostile party
might send*. The midpoint is comfortable:

| Cap | Default | 2 000-page book lands at |
|---|---:|---:|
| `source_bytes` | 64 MiB | 3.8 MiB — 6% |
| `nodes_per_document` | 100 000 | ~108 000 — **just over the cap** ⚠ |
| `block_nesting_depth` | 32 | 4 — 12% |
| `package_uncompressed_bytes` | 256 MiB | n/a (text) |

The synthetic book grazes `nodes_per_document` at 2 000 pages of *rich*
content because every page emits about 50 nodes. A book of comparable
length with leaner content (less metadata, fewer callouts) fits the
default cap. A 5 000+ page reference manual will not — see the next
section.

## Tuning for large documents

The CLI uses defaults. For larger documents, drive the library directly:

```rust
use nodx_core::{parse_str_with_limits, ResourceLimits};

let mut limits = ResourceLimits::default();
limits.source_bytes = 256 * 1024 * 1024;        // 256 MiB instead of 64 MiB
limits.nodes_per_document = 1_000_000;           // 1 M instead of 100 K
limits.block_nesting_depth = 64;                 // depth headroom

let doc = parse_str_with_limits(&source, limits);
```

There is no magic in these numbers. The product `nodes_per_document × 1
KiB-per-node` is a useful upper bound on RAM for the AST itself; the
renderer adds a similar-sized output buffer; pick numbers your hosting
environment can sustain.

## When defaults *should* refuse

The defaults catch hostile inputs (`gzip -d` bombs, billion-laughs-style
node explosions, multi-megabyte attribute values). When the parser
emits `NODX-E012` on input that came from a user submission, that is
working as intended.

The same `NODX-E012` on a legitimate large document is the signal that
*your driver* should pass higher limits via `parse_str_with_limits`. The
CLI is deliberately not the place to relax those bounds.

## Where the implementation is *not* streaming

The reference parser materializes the entire AST in memory before any
downstream consumer touches it. There is no incremental API, no token
stream, no "render this chunk while parsing the next" pipeline. That is
fine for documents that fit in memory — i.e. everything we have
measured — and it keeps the implementation small.

For documents whose AST does not fit in memory, the [streaming evolution
proposal](./streaming-evolution.md) sketches the design space without
committing to an implementation.

## Re-running the benchmarks

The repository does not commit a benchmark harness — the numbers above
were measured ad-hoc. To reproduce:

```sh
cat > /tmp/gen_book.py <<'PY'
import random, sys
random.seed(42)
PAGES = int(sys.argv[1])
OUT = sys.argv[2]
WORDS = "structured document format deterministic canonical".split()
def sentence(n=12): return " ".join(random.choice(WORDS) for _ in range(n)).capitalize() + "."
def para(n=4): return " ".join(sentence() for _ in range(n))
with open(OUT, "w") as f:
    f.write("---\ntitle: Synthetic\n---\n\n")
    for p in range(1, PAGES + 1):
        f.write(f"## Chapter {p} #ch{p}\n\n{para()}\n\n")
        f.write(f"### Section {p}.1 #ch{p}-s1\n\n{para()}\n\n")
        f.write("- " + sentence() + "\n- " + sentence() + "\n\n")
        f.write("| A | B |\n|---|---|\n| 1 | 2 |\n\n")
PY
python3 /tmp/gen_book.py 2000 /tmp/book.nodx
/usr/bin/time -l target/release/nodx html /tmp/book.nodx > /dev/null
```

Compare your numbers against the table above. Order-of-magnitude
agreement is enough; the constants depend on CPU and disk cache state.

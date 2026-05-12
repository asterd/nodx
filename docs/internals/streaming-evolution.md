# Streaming evolution — design notes

**Status:** exploratory. Not committed for any milestone.
**Scope:** parser, renderer, NCP projector.
**Out of scope:** validator (already streams), package reader (already
in-memory by design), signing (operates on hashes, not bytes).

## Motivation

The reference implementation parses and renders a 2 000-page book
comfortably in under a second with an 80 MB resident set. A 10 000-page
input is parsed in under a second but consumes ~400 MB. The curve is
linear, with a slope of about 20× the source size in peak RSS.

This is fine for every documented use case and most aspirational ones.
It stops being fine in three scenarios:

1. **Multi-tenant servers** that hold many open documents in memory and
   need each one's footprint to be modest, not "fits in the box".
2. **Embedded readers** that target devices with hard memory ceilings
   (kiosks, e-readers, in-browser WASM bundles).
3. **Documents that legitimately do not fit in memory** — a generated
   reference manual for an OS, a legal corpus, an indexed Bible-sized
   work with extensive annotations.

The current architecture cannot serve those scenarios without changes.

## Goal

Move the parser, HTML renderer, and NCP projector to a *streaming*
architecture: the host application drives a pump, the implementation
exposes tokens or events, and downstream consumers (writer, indexer,
hasher) materialize only what they need.

The non-goal is to *replace* the in-memory pipeline. It works, it is
simple, it is fast. The streaming variant must coexist with it as a
separate execution mode.

## What "streaming" means here

Three layers, each independently useful:

### Layer 1 — lexical / block-event stream

The parser emits a token-like event stream:

```rust
enum BlockEvent<'a> {
    FrontMatter(YamlMap<'a>),
    BlockStart { name: &'a str, attrs: Attrs<'a>, colons: u8 },
    BlockEnd { name: &'a str },
    Heading { level: u8, inlines: InlineStream<'a> },
    Paragraph(InlineStream<'a>),
    ListStart { kind: ListKind },
    ListItem(InlineStream<'a>),
    ListEnd,
    TableStart,
    Row(Vec<InlineStream<'a>>),
    TableEnd,
    LiteralBlock { name: &'a str, body: &'a str },
    Diagnostic(Diagnostic),
}
```

The driver pulls events and decides what to do with them. The renderer
can write HTML to a `std::io::Write` as events arrive. The NCP projector
can stream JSON to disk. A hasher can update a SHA-256 with the
canonical bytes of each event.

The AST builder becomes one possible consumer of the event stream
instead of being baked into the parser.

### Layer 2 — pull-based input

The parser today takes `&str`. A streaming parser takes a `BufRead` (or
WASM-friendly equivalent) and a *bounded* line buffer. Lines longer than
`line_length` are still rejected; longer-than-RAM line streams are
impossible to express by construction.

The parser maintains its own line buffer, never materializes the full
source, and never indexes into the source by absolute offset (today the
block parser does `self.lines[start..self.pos].join("\n")` to recover
the body of a literal block — this pattern needs replacing with an
explicit "begin literal capture" / "end literal capture" event pair).

### Layer 3 — semantic projection

NCP is the easiest consumer to stream: per-node hashing is local, and
the output is line-oriented JSON. The HTML renderer is medium-effort:
nested blocks expect to know their children before they emit their open
tag (`<aside class="…">`), but the renderer can buffer the open tag's
attribute string and flush as soon as the *first* child arrives.

The hardest consumer is the agent SDK: applying a `Batch` requires the
prior state's hash, and the prior state is by definition the full AST.
Batches stay AST-based.

## What stays the same

- The canonical AST shape. Streaming is an *execution* concern; the
  observable output is identical bytes.
- The diagnostic codes and severities. A streaming parser emits the
  same `NODX-Exxx` events as the in-memory one.
- The resource limits. Streaming changes *when* a limit is hit, not
  *whether*.
- The CLI surface. `nodx html` continues to take a path and write a
  file. Internally it picks the streaming pipeline for inputs above a
  threshold; the user does not have to know.

## Compatibility plan

1. **Phase 1** — add `parse_events` as a *new* API in `nodx-core`
   alongside `parse_str`. Both produce the same canonical AST when
   collected to completion.
2. **Phase 2** — port the HTML renderer and NCP projector to consume
   either an AST or an event stream. Default to AST for backward
   compatibility.
3. **Phase 3** — switch the CLI to select streaming above an opt-in
   threshold (`limits.source_bytes / 4`, say). Keep AST as the
   non-streaming default.
4. **Phase 4** — once the streaming pipeline passes the conformance
   sweep byte-for-byte, deprecate nothing; document streaming as the
   recommended path for documents over a configurable size.

There is no v2.0 in this. Streaming is purely additive.

## Conformance impact

The conformance bundle compares bytes of canonical AST, NCP, and
diagnostics. Streaming must reproduce those exact bytes when its output
is collected. The conformance runner can run each fixture through both
the AST and the event pipelines and assert byte-equality between them.

## What we are not adding

- **Incremental reparsing.** Already covered by `nodx-cst`'s patch
  ranges. Streaming and incremental are different problems.
- **A custom binary on-disk format.** NODX is text-first. A binary AST
  cache for "huge documents loaded many times" is a separate, much
  smaller proposal.
- **Multi-threaded parsing.** Linear parsing of a single document is
  fast enough that thread coordination would dominate. Multiple
  documents in parallel is already possible at the process level.

## When this evolves

A serious attempt is worth doing when a real user reports a real
document that does not fit. Until then, the in-memory pipeline serves
the documented and aspirational use cases without complication. The
design space sketched here is meant to make sure we know which way we
would go, not to be a roadmap.

## References to current code

The places that will need attention when this becomes work:

- [`crates/nodx-core/src/block_parser.rs`](../../crates/nodx-core/src/block_parser.rs)
  — line-buffer ownership, literal block capture, diagnostic emission.
- [`crates/nodx-core/src/canonical.rs`](../../crates/nodx-core/src/canonical.rs)
  — serializer that emits the same byte order in either execution mode.
- [`crates/nodx-render-html/src/lib.rs`](../../crates/nodx-render-html/src/lib.rs)
  — currently writes to a `String`; a streaming variant should take
  `&mut dyn Write`.
- [`crates/nodx-ncp/src/lib.rs`](../../crates/nodx-ncp/src/lib.rs)
  — already mostly stateless per node; smallest delta of the three.
- [`scripts/run_conformance.sh`](../../scripts/run_conformance.sh)
  — add an `--engine streaming` mode that runs the same fixtures through
  the event pipeline and asserts byte-equality.

A reasonable estimate for the work is **two engineer-weeks** for layer 1
and layer 3 (parser events + NCP), and **another two** to land the
streaming HTML renderer cleanly. The cost-to-benefit is *very* dependent
on whether anyone needs it.

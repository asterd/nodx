# Using the CLI

`nodx` is the reference command-line tool. Everything you can do with the
library is exposed here, with stable exit codes that fit into CI.

## Invocation shape

```text
nodx <subcommand> <file> [options]
```

Subcommands operate on a single path (text or packaged NODX). They either
print to stdout or write a single output file when `-o` is given. Errors go
to stderr. Exit codes are predictable:

| Code | Meaning |
|---|---|
| `0` | Success. |
| `1` | I/O error or invalid CLI usage. |
| `2` | Parse, validation, or security failure. |
| `3` | The document requires a capability this build does not support (`NODX-E024`). |

## Subcommands at a glance

| Subcommand | What it does |
|---|---|
| `inspect` | Quick human summary: profiles, headings, blocks, diagnostics. |
| `ast` | Print the canonical AST as JSON. |
| `validate` | Run the validator and emit diagnostics (text or JSON). |
| `html` | Render to a complete, self-contained HTML document. |
| `tui` | Render to a terminal-friendly representation. |
| `ncp` | Emit the agent-readable NCP projection. |
| `semantic` | Emit compact semantic text for LLM context and search. |
| `integrity` | Print the lightweight front matter integrity digest for this document. |
| `package inspect` | Show manifest, entries, and check declared sizes/digests. |
| `package verify` | Recompute every entry's SHA-256 and compare to the manifest. |
| `export pdf` | Render a paged HTML pipeline preview suitable for headless print. |
| `export docx` | Emit a `.docx` preview with a loss report. |
| `export pptx` | Emit a `.pptx` preview with a loss report. |

## Common patterns

### Render to HTML

```sh
nodx html doc.nodx > doc.html
```

The output is one self-contained HTML5 page, no external JS, no CDN dependencies,
with a strict `Content-Security-Policy` baked into the `<meta>` header.

### Render to HTML with a custom theme

Themes resolve from the front matter (`theme: docs`, etc.) or, for packaged
documents, from `.nods` files declared in `manifest.yaml`. The CLI does not
take a `--theme` argument — the document decides.

### Validate in CI

```sh
nodx validate doc.nodx --format json | tee diagnostics.json
```

The validator never writes files. Pipe the JSON into your CI's annotations
system and key off the `code` field for stable rules.

### Diff two documents semantically

The canonical AST is byte-stable. Two documents that mean the same thing
produce the same bytes:

```sh
nodx ast a.nodx | jq -S . > a.ast.json
nodx ast b.nodx | jq -S . > b.ast.json
diff -u a.ast.json b.ast.json
```

(The `jq -S .` is unnecessary for correctness but produces a nicer diff.)

### Extract structured outline for agents

```sh
nodx ncp doc.nodx | jq .
```

The NCP projection is a node-level extraction designed for retrieval and
batch agent workflows. It preserves node paths, IDs, hashes, attributes,
children, and resolved navigation entries.

### Add light integrity metadata

```sh
nodx integrity doc.nodx
```

Copy the printed `sha256-...` value into front matter:

```yaml
integrity:
  alg: sha256
  scope: canonical-ast
  value: sha256-...
```

The validator recomputes the same digest with the `integrity` field excluded.
If the document changes afterwards, validation emits `NODX-E028`.

### Extract compact LLM context

```sh
nodx semantic doc.nodx
```

Semantic text keeps readable source content and fallback children while
excluding styles, table-of-contents nodes, page breaks, computed layout, and
custom component template output.

### Run the full conformance sweep

```sh
sh scripts/run_conformance.sh
```

Compares the Rust and JavaScript reference parsers against every committed
fixture and writes `target/conformance-report.json`. A non-zero exit
indicates a divergence — usually a regression you want to know about.

## Resource limits from the CLI

The CLI uses default limits from `ResourceLimits::default()`. For documents
that legitimately need more (a 5000-page manual, a 250 MiB packaged
artifact), call the library directly with custom limits. The CLI is
intentionally opinionated about safety.

## Output stability

`nodx ast`, `nodx ncp`, `nodx semantic`, and `nodx html` are designed to be byte-stable
across runs of the same binary on the same input. That means:

- `nodx html` output makes a good CI artifact you can diff.
- Cache invalidation can hash the canonical AST instead of the source.
- Two parallel runs produce identical bytes; you can dedupe outputs by hash.

NODX 1.0 projections do not add timestamps, random IDs, or host layout state.

## Exit code cheat sheet

```sh
nodx validate good.nodx ; echo $?     # 0
nodx validate bad.nodx ; echo $?      # 2
nodx html missing.nodx 2>/dev/null ; echo $?   # 1
nodx html agent-required.nodx ; echo $?        # 3
```

Wire these into your CI rules directly. The codes are part of the public
contract.

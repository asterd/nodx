# Quickstart

Your first NODX document in five minutes. No reading, just typing.

## Install the CLI

Clone the repository and build the reference binary:

```sh
git clone https://github.com/<your-org>/nodx.git
cd nodx
cargo build --release -p nodx
```

The binary lives at `target/release/nodx`. Put it on your `$PATH` if you plan
to use it day to day.

> The reference implementation only depends on the Rust standard library. No
> network calls, no `build.rs` magic, no codegen. A working `cargo` is enough.

## Write a document

Create `hello.nodx`:

```nodx
---
title: Hello NODX
theme: web
---

# Hello, world #intro

This is **structured text** with a safe [link](https://example.com).

::note {type="info"}
NODX gives you Markdown's flow with a deterministic AST underneath.
::
```

## See it render

```sh
nodx html hello.nodx > hello.html
open hello.html   # or: xdg-open / start, depending on your OS
```

You should see a complete, self-contained HTML page with a sensible default
theme, a heading anchor at `#intro`, and a styled callout.

## Check it

```sh
nodx validate hello.nodx --format text
```

If the document is valid, the command prints nothing and exits with `0`. If
the document has a problem, you get a structured diagnostic per line and a
non-zero exit code:

| Exit code | Meaning |
|---|---|
| `0` | Document is valid. |
| `1` | I/O or CLI argument error. |
| `2` | Parse, validation, or security failure. |
| `3` | The document requires a capability this build does not support. |

The same diagnostics, in JSON, are available with `--format json`:

```sh
nodx validate hello.nodx --format json | jq .
```

## Inspect the canonical AST

```sh
nodx ast hello.nodx | jq '.body[0]'
```

The output is byte-stable. Two implementations parsing the same input must
produce the same bytes. That is what makes NODX safe for CI, diffing, and
machine consumers.

## Where to go next

- The [Syntax tour](./02-syntax-tour.md) — every construct, side by side.
- The [Authoring guide](./03-authoring.md) — patterns for real documents.
- The [Cookbook](../cookbook/) — recipes for the questions you will actually have.
- The [Reference](../reference/) — the boring, exhaustive description of every
  CLI command, every AST node, every diagnostic code.

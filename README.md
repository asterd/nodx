<p align="left">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/assets/logo-dark.svg">
    <img alt="NODX" src="docs/assets/logo.svg" width="280">
  </picture>
</p>

**Structured documents for people, tools, and AI agents.**

NODX is a text-first document format. It keeps the parts of Markdown
that make documents easy to read, and adds the parts Markdown leaves to
convention: a canonical AST, deterministic JSON output, a fail-closed
security model, packaged local assets, stable navigation, and an
agent-readable projection.

Use it when a document has to be **readable as plain text, rendered
safely, checked in CI, exported to HTML or PDF, and consumed by
software that does not get to guess** what the author meant.

> The normative contract is [NODX-RFC-0001](./NODX-RFC-0001.md). The
> reference implementation in this repository is the conformance gauge.

## Try it online

| Destination | Use it for | Link |
|---|---|---|
| Documentation | Read the guide, reference, cookbook, and internals. | [Open docs](https://asterd.github.io/nodx/) |
| Playground | Edit real NODX examples and inspect render, diagnostics, AST, NCP, semantic text, and packages. | [Open playground](https://asterd.github.io/nodx/playground/) |

## In sixty seconds

Write a document:

```nodx
---
title: My first NODX document
theme: web
---

# Hello NODX #intro

This is **structured text** with a safe [link](https://example.com).
Reviewer: {{reviewer}}.

::note {type="info"}
Unknown renderers keep this fallback content readable.
::
```

Render it:

```sh
cargo build --release -p nodx
target/release/nodx html my-doc.nodx > my-doc.html
target/release/nodx validate my-doc.nodx --format json
target/release/nodx ncp my-doc.nodx > my-doc.ncp.json
target/release/nodx semantic my-doc.nodx > my-doc.semantic.txt
```

Four commands. Four byte-stable outputs. No CDN, no network, no
hidden state.

## Why not just Markdown?

| Capability                      | Markdown | AsciiDoc | LaTeX | NODX |
|---------------------------------|:--------:|:--------:|:-----:|:----:|
| Readable as plain text          | yes      | yes      | partial | yes |
| Canonical, byte-stable AST      | no       | partial  | no    | yes |
| Safe for untrusted input        | no       | no       | no    | yes |
| Built-in semantic navigation    | partial  | yes      | partial | yes |
| Packaged ZIP with local assets  | no       | no       | no    | yes |
| Agent-readable projection       | no       | no       | no    | yes |
| Conformance profiles            | no       | partial  | no    | yes |
| Diagnostics with stable codes   | no       | no       | no    | yes |

Markdown is wonderful for prose and terrible as an interchange format.
NODX is the same readability with the contract Markdown never had.

## What you get in this repository

- **Reference implementation in Rust**, 12 focused crates:
  parser/validator/renderer/exporter, URL and style auditors, package
  reader, NCP projector, signing, CST for editors, agent SDK, CLI.
  Every crate is `#![forbid(unsafe_code)]`.
- **JavaScript and Python parsers** under [`packages/`](./packages/) —
  both produce the same canonical AST as the Rust reference and are
  exercised by the conformance suite. Their package readers enforce the
  same safe-path, manifest, digest, and stored-ZIP integrity checks used by
  the browser-facing package surface.
- **Conformance bundle** at
  [`spec/conformance/v1.0/`](./spec/conformance/v1.0/) — every fixture
  with its expected AST, NCP, diagnostics, and HTML outputs.
- **Hostile-input corpus** under [`spec/tests/security/`](./spec/tests/security/)
  — YAML, NODS, URL, packaging, and XSS test cases.
- **Showcase documents** under [`examples/`](./examples/) — minimal,
  intermediate, advanced, packaged, i18n, print, and a documentation
  layout demo.
- **Editor starter integrations** for VS Code, Sublime, and Notepad++
  under [`editors/`](./editors/).
- **Apps**: a web playground in [`apps/web/`](./apps/web/) and a Python
  desktop viewer in [`apps/desktop/`](./apps/desktop/).

## Documentation

The full documentation lives under [`docs/`](./docs/) and is published
as a navigable site under [`site/`](./site/). Read it in three ways:

- **Hosted docs**:
  [Open the documentation site](https://asterd.github.io/nodx/).
- **Hosted playground**:
  [Open the browser playground](https://asterd.github.io/nodx/playground/).
  Both are built and deployed by `.github/workflows/docs.yml` on every
  push after GitHub Pages is enabled with Source: GitHub Actions.
- **Locally** as a site:

  ```sh
  python3 scripts/build_site.py
  python3 -m http.server 8765 -d site
  open http://127.0.0.1:8765/
  ```

- **As plain Markdown** in your editor: the source files under
  [`docs/`](./docs/) are the same files the site renders.

### Where to start

| You are… | Read |
|---|---|
| Brand new to NODX | [Quickstart](./docs/guide/01-quickstart.md) and the [Syntax tour](./docs/guide/02-syntax-tour.md). |
| Writing a real document | The [Authoring guide](./docs/guide/03-authoring.md). |
| Wiring NODX into a tool or CI | The [CLI guide](./docs/guide/05-cli.md) and the [Cookbook](./docs/cookbook/index.md). |
| Implementing your own parser | The [conformance reference](./docs/reference/conformance.md) and the [AST reference](./docs/reference/ast.md). |
| Auditing for security | The [security model](./docs/internals/security-model.md) and [`SECURITY.md`](./SECURITY.md). |
| Sizing the system | The [scalability notes](./docs/internals/scalability.md). |

The full [docs index](./docs/README.md) lists every page.

## Build and verify

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --release     # all Rust crates
node --test packages/nodx-js/test/*.mjs
python3 -m pytest packages/nodx-py/tests -q
sh scripts/run_conformance.sh        # Rust ↔ JavaScript parity over every fixture
sh scripts/verify_conformance_package.sh
```

The conformance script writes `target/conformance-report.json` for CI
consumption. A non-zero exit code from either script is a real failure —
they do not tolerate divergence.

## CLI cheat sheet

```sh
target/release/nodx ast doc.nodx
target/release/nodx validate doc.nodx --format json
target/release/nodx html doc.nodx > doc.html
target/release/nodx tui doc.nodx
target/release/nodx ncp doc.nodx
target/release/nodx semantic doc.nodx
target/release/nodx package inspect bundle.nodx
target/release/nodx package verify  bundle.nodx
target/release/nodx export pdf  doc.nodx -o doc.pdf
target/release/nodx export docx doc.nodx -o doc.docx
target/release/nodx export pptx slides.nodx -o slides.pptx
```

Exit codes follow the RFC:

| Code | Meaning |
|---|---|
| `0` | Success. |
| `1` | I/O or CLI usage error. |
| `2` | Parse, validation, or security failure. |
| `3` | The document requires a capability this build does not support (`NODX-E024`). |

Full reference: [docs/guide/05-cli.md](./docs/guide/05-cli.md).

## Scalability — measured

Synthetic book benchmarks on the reference release build:

| Input | Source | Operation | Wall time | Peak RSS |
|---|---:|---|---:|---:|
| 2 000 pages | 3.8 MiB | `nodx html` | 0.16 s | 80 MiB |
| 5 000 pages | 9.6 MiB | `nodx html` | 0.40 s | 195 MiB |
| 10 000 pages | 19 MiB | `nodx html` | 0.79 s | 386 MiB |

Linear in input size, ~20× source-bytes RAM. Documents past 2 000 pages
of *rich* content will graze the default `nodes_per_document` cap; raise
it via the library API. The [scalability notes](./docs/internals/scalability.md)
have the full picture and instructions for reproducing the numbers; the
[streaming evolution proposal](./docs/internals/streaming-evolution.md)
sketches what would change for documents that do not fit in memory.

## Security

NODX processors are expected to **fail closed** on untrusted input: no
script execution, no default network fetches, no package extraction to
disk, strict resource limits, safe URL/path policy, context-aware
escaping. See [`SECURITY.md`](./SECURITY.md) for the policy and the
[security model](./docs/internals/security-model.md) for the threat
model.

## License

Licensed under the **MIT License** — see [`LICENSE`](./LICENSE) and
[`NOTICE`](./NOTICE). You may use, modify, and redistribute the code under
the License. Contributions are accepted under the same terms.

## Contributing

Before opening a PR or filing a security report, read:

- the [implementer guide](./docs/IMPLEMENTER_GUIDE.md);
- the [conformance reference](./docs/reference/conformance.md);
- the [security policy](./SECURITY.md).

If you find a fixture where the Rust and JavaScript parsers produce
different bytes for the same input, that is the bug we most want to
hear about. Open an issue with both outputs attached.

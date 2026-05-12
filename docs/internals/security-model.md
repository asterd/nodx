# Security model

NODX is built to handle *untrusted* document input safely. This page
describes the threat model and the load-bearing properties that keep the
implementation faithful to it.

## Threat model

NODX is read by software that does not get to choose its inputs:

- CI runners parsing PR descriptions stored as `.nodx`.
- Web services rendering documents uploaded by anonymous users.
- Knowledge bases indexing third-party content for retrieval.
- Agents consuming external documents as facts.

The attacker assumed in this model can:

- Submit arbitrary bytes — well-formed NODX, malformed NODX, or
  not-NODX at all.
- Submit a ZIP package with a manipulated manifest, a zip bomb, or a
  path traversal entry.
- Submit a document with hostile YAML, hostile NODS, or hostile inline
  HTML attempts.
- Submit a document at the boundary of every resource limit.
- Provide a URL or `data:` URI that points at internal services.

The attacker cannot:

- Compromise the host operating system, the build pipeline, or the
  filesystem outside the process.
- Force the implementation to execute arbitrary code via the document.
- Make the implementation fetch network resources by accident.

## Properties the implementation must preserve

These properties are the load-bearing safety claims. A change that
weakens any of them needs an explicit discussion.

1. **No execution.** No path in any crate executes document content as
   code. There is no JavaScript runtime, no shell out, no `eval`, no
   `Function(…)`, no template engine with a code escape.
2. **No network I/O.** No crate makes outbound network calls. The
   parser, the validator, the renderer, the package reader, and the
   exporter are all offline.
3. **No filesystem writes during read.** Parsing, validating, rendering,
   projection, and package opening never write files. Export commands
   write only the artifact the user named.
4. **No `unsafe` Rust.** Every crate carries
   `#![forbid(unsafe_code)]`.
5. **Bounded resources.** Every expensive operation checks
   [`ResourceLimits`](../reference/limits.md) before doing work. Limits
   that are not enforced are documented as such.
6. **Context-aware escaping.** The renderer never relies on
   block-level escaping. Every value is escaped for its destination
   context (text, attribute, URL, style).
7. **Fail closed.** Unknown profile → reject. Unknown component →
   safe fallback. Unsafe URL → blocked link. Malformed manifest →
   refuse to open the package. Stylesheet rule outside the safe subset
   → drop the rule, not the document.

## Hardening surface

| Surface | Crate | What it catches |
|---|---|---|
| UTF-8 + BOM + U+0000 | `nodx-core` | Encoding tricks. `NODX-E002`, `NODX-E018`. |
| YAML safe subset | `nodx-core::front_matter`, `nodx-package::manifest` | Anchors, aliases, tags, merge keys, duplicates, multi-doc, timestamps, binary. `NODX-E019`. |
| Resource limits | `nodx-core::limits` | Bytes, lines, depth, nodes, attribute values. `NODX-E012`. |
| URL classifier | `nodx-url` | `javascript:`, `vbscript:`, `file:`, oversized `data:`, package path escape. `NODX-E020`. |
| NODS auditor | `nodx-style` | `expression()`, remote `@import`, `position: fixed`, hover states, network `url()`. `NODX-E027`. |
| Package reader | `nodx-package` | Zip bombs, path traversal, manifest lies, nested zips, SHA-256 mismatches. `NODX-E010`, `NODX-E012`, `NODX-E019`, `NODX-E021`. |
| HTML escaper | `nodx-render-html` | Attribute breakout, style breakout, URL injection. |
| Signature verifier | `nodx-sign` | Forged or rebound signatures. `NODX-E017`. |

Every diagnostic in this surface is also a corpus entry under
`spec/tests/security/`. The conformance sweep runs the corpus on every
release-candidate build.

## What is *not* part of the model

- **DRM.** NODX does not authenticate or restrict readers. Signatures
  prove authorship; they do not gate access.
- **Steganography.** A NODX document is plain text or a plain ZIP. There
  is no hidden side-channel.
- **Sandbox.** The crates do not sandbox themselves. A host that wants
  a hard isolation boundary should run NODX inside a process with
  cgroups, seccomp, or an equivalent OS facility.
- **Side-channel resistance.** Hashing uses standard SHA-256; signature
  verification uses standard ECDSA. We do not promise constant-time
  behavior beyond what the standard library and the verifier provide.

## Reporting a vulnerability

See [`SECURITY.md`](../../SECURITY.md) at the repository root for the
current reporting channel and the disclosure expectations. The short
version: open a private channel to the maintainer, include a minimal
reproducer, and we will respond before any public disclosure.

We are particularly interested in:

- Inputs that crash the parser, the renderer, or the package reader.
- Inputs that bypass `NODX-E020`, `NODX-E027`, or `NODX-E021`.
- Documents whose canonical AST differs between the Rust and JavaScript
  parsers.

Less interesting:

- "The validator emits diagnostics that look ugly in CI." That is a
  product issue, not a security issue.
- "I can write a slow document that the parser accepts at the limit."
  Look at `nodes_per_document` and friends; if they are not enough,
  open a discussion about defaults.

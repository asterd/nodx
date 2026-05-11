# NODX Threat Model

This threat model records the NODX 1.0 security assumptions, trust boundaries,
and release-gate mitigations for the reference implementation.

## Assets

- User document contents.
- Local filesystem paths and file contents.
- Package asset integrity.
- Canonical AST and NCP hashes.
- Renderer output safety.
- Host process availability and memory.
- Agent and indexer context integrity.

## Trust Boundaries

- External bytes entering a parser or package reader.
- Front matter entering the metadata model.
- Attribute values entering validators, URL policy, style policy, renderers, and
  projections.
- Package manifests and package entry paths.
- Local assets referenced by documents.
- Renderer output contexts such as HTML text, attributes, URLs, and styles.
- NCP output consumed by agents or retrieval systems.

## Adversaries

- An author of a malicious `.nodx` text file.
- An author of a malicious `.nodx` package.
- A document converted from an untrusted external format.
- A compromised package asset.
- A caller attempting denial of service through oversized or deeply nested
  inputs.

## Primary Threats

| Threat | Default mitigation |
|---|---|
| Script or active-content execution | No executable document content; escape rendered output. |
| Path traversal | Reject absolute paths, `..`, backslashes, and paths escaping package root. |
| Unsafe URL schemes | Reject unsafe schemes with `NODX-E020`. |
| Remote resource exfiltration | No network fetches by default. |
| ZIP bombs or package expansion abuse | Enforce package size, count, entry size, ratio, and nested ZIP limits. |
| Parser denial of service | Enforce source, line, node, nesting, and AST memory limits. |
| Style breakout | Reject forbidden NODS constructs and style-context breakouts. |
| Misleading agent context | Deterministic NCP hashes and explicit loss diagnostics. |
| Unsupported required capability | Fail closed with `NODX-E024` and CLI exit `3`. |

## Boundary-Specific Notes

### Text Parser

Untrusted bytes enter through `parse_bytes`. The parser must validate UTF-8,
reject U+0000, bound source size, and produce deterministic diagnostics instead
of panicking on malformed input.

### Front Matter

Front matter is treated as data, not executable YAML. The safe subset forbids
references, tags, object construction, duplicate keys, multiple documents, and
non-finite numeric values.

### URLs and Assets

Resource references cross from document text into host or package resource
policy. The default policy permits safe fragments and package-relative paths,
allows only documented link schemes, rejects dangerous schemes, and does not
fetch remote resources.

### Packages

ZIP package bytes cross a filesystem-like boundary. The reader must validate
central directory metadata, reject traversal and special files, enforce limits,
verify manifest metadata, and expose only read-only in-memory access.

### Rendering

HTML rendering crosses into browser interpretation. Renderers must escape by
context, omit unsafe style rules, reject active content, and include restrictive
browser policy where possible.

### Agent Consumption

NCP output crosses into retrieval and agent contexts. Canonical AST and NCP
hashes must be deterministic, unsupported capabilities must be visible through
diagnostics, and lossy projections must not pretend to preserve unsupported
semantics.

## Non-Goals for 1.0

- Trust establishment for signatures.
- Agent mutation authorization.
- Collaborative synchronization security.
- Sandboxed active widgets or plugins.
- Native PDF/DOCX/PPTX exporter threat modeling.

## Residual Risks and Future Work

1. Grow the committed security corpus to the numeric targets in
   `NODX_1.0_Evolution_Plan.md`.
2. Complete and record the release-candidate fuzz budget from `fuzz/README.md`.
3. Add broader package bomb archives and traversal variants.
4. Expand renderer escaping fixtures by output context.
5. Document host policy override rules and their tests.
6. Threat-model signatures, agent mutation, lossless editing, and native export
   profiles when those future profiles enter scope.

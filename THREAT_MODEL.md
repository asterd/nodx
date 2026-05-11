# NODX Threat Model

This is the initial threat-model skeleton for NODX 1.0. It records the security
assumptions that future implementation waves must satisfy and test.

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

## Non-Goals for 1.0

- Trust establishment for signatures.
- Agent mutation authorization.
- Collaborative synchronization security.
- Sandboxed active widgets or plugins.
- Native PDF/DOCX/PPTX exporter threat modeling.

## Required Future Work

Before NODX 1.0 release:

1. Add security corpus fixtures for every error code that guards a boundary.
2. Add fuzz targets for text parsing, front matter, attributes, package parsing,
   URL policy, style validation, and canonical serialization.
3. Add package bomb and traversal fixtures.
4. Add renderer escaping fixtures by output context.
5. Document host policy override rules and their tests.

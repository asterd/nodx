# Security Policy

NODX is designed for offline-first, fail-closed processing of untrusted document
input. This policy covers the NODX 1.0 reference implementation release gate.

## Supported Versions

| Version | Security status |
|---|---|
| `nodx/1.0` | Stable reference contract defined by `NODX-RFC-0001.md`. |

## Baseline Rules

Processors handling untrusted input must:

1. validate UTF-8 before parsing source text;
2. enforce resource limits before expensive work;
3. reject U+0000 and handle BOMs according to policy;
4. never execute document content;
5. never fetch remote resources by default;
6. block unsafe paths and URL schemes;
7. preserve safe fallback content for unknown components;
8. escape rendered output by context;
9. avoid writing files during read, parse, validate, render, or projection;
10. report structured diagnostics for security-relevant rejection.

## Explicitly Forbidden by Default

- Scripts, macros, plugins, and active content.
- Remote styles, imports, includes, and resource fetches.
- `file://`, `javascript:`, and `vbscript:` references.
- Package extraction to disk during read.
- Inline or active SVG in NODX 1.0.
- Inline MathML in NODX 1.0.
- Agent mutations in NODX 1.0.

## Implemented Security Boundaries

- `nodx-core` validates UTF-8 bytes before text parsing and rejects U+0000.
- Front matter accepts only the documented YAML safe subset and rejects anchors,
  aliases, explicit tags, merge keys, duplicate keys, multiple documents,
  timestamp-like scalars, binary tags, custom objects, and non-finite numbers
  with `NODX-E019`.
- `nodx-url` centralizes URL classification, resource limits, and package path
  normalization.
- `nodx-render-html` escapes HTML text, attributes, URLs, and style content by
  context and emits a restrictive CSP.
- `nodx-style` audits and sanitizes the safe NODS subset; forbidden constructs
  produce `NODX-E027` diagnostics and unsafe rules are omitted from rendered
  HTML.
- `nodx-package` reads stored ZIP packages into an in-memory read-only
  `PackageFs`, verifies manifest-listed sizes and SHA-256 digests, rejects path
  traversal and special files, and never extracts package content to disk.
- Unsupported required profiles fail closed with `NODX-E024` and CLI exit code
  `3`.

## Security Corpus

Committed security inputs:

- `spec/tests/security/url-policy.tsv`
- `spec/tests/security/yaml-hostile`
- `spec/tests/security/nods-hostile`
- `spec/tests/security/xss`
- `spec/tests/security/package-corpus.md`

The corpus is active and run through unit, renderer, package, validator, or
conformance checks as appropriate. Larger numeric corpus targets and fuzz
budgets remain release-readiness work, not changes to the RFC contract.

## Fuzzing

Fuzz target entry points are published under `fuzz/`. Local and release-branch
commands are documented in `fuzz/README.md`.

The stable release gate is a reproducible smoke budget, not an open-ended
24 CPU-hour target: every fuzz target must pass `scripts/fuzz_smoke.sh` with
at least 5 000 libFuzzer runs per target on the release commit, and the weekly
`Fuzz smoke` workflow must be green or have a documented infrastructure-only
failure. Maintainers can raise `NODX_FUZZ_RUNS` for release candidates without
changing the contract. Accepted fuzz findings must be recorded in this file or
in a linked advisory before a stable release is tagged.

## Reporting Vulnerabilities

Report suspected vulnerabilities privately through GitHub Security Advisories
when available for the repository. If private advisories are unavailable, email
the maintainer security contact listed on the repository profile or request a
private channel from the maintainer before sharing exploit details.

Include:

1. affected command or API;
2. minimal input file or package;
3. observed behavior;
4. expected safe behavior;
5. host OS and build information.

Do not include secrets, private documents, or production data in reports.

Maintainers should acknowledge reports within 7 calendar days, provide an
initial triage decision within 14 calendar days, and publish a fix, mitigation,
or advisory once affected supported versions are understood.

## Current Accepted Findings

No accepted security findings are documented for this wave.

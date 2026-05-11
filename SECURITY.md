# Security Policy

NODX is designed for offline-first, fail-closed processing of untrusted document
input. This file is a skeleton for the 1.0 security policy and will be expanded
as the implementation reaches the security milestones in
`NODX_1.0_Evolution_Plan.md`.

## Supported Versions

| Version | Security status |
|---|---|
| `nodx/1.0` | Target contract, not released yet. |
| `nodx/0.1` | Reference implementation draft, not a stable security release. |

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

## Reporting Vulnerabilities

This repository does not yet publish a stable vulnerability disclosure process.
Until one is added, report suspected vulnerabilities through the repository
maintainer channel and include:

1. affected command or API;
2. minimal input file or package;
3. observed behavior;
4. expected safe behavior;
5. host OS and build information.

Do not include secrets, private documents, or production data in reports.

## Security Test Status

The repository does not yet include the full 1.0 security corpus or fuzzing
targets. Those are required before a stable 1.0 release.

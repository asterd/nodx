# Diagnostic codes

Every diagnostic produced by the reference implementation has a stable
`NODX-Exxx` code, a severity, a human message, and a source location when
one is available. The codes never change across patch releases of the
spec, which is what lets you pin lints in CI.

## Severities

| Severity | Exit code | Meaning |
|---|---|---|
| `info` | `0` | Suggestion or informational note. |
| `warning` | `0` | Likely problem worth fixing but not blocking. |
| `error` | `2` | The document is malformed but the parser kept going. |
| `fatal` | `2` | The parser stopped at this point. The AST may be partial. |

`NODX-E024` ("Required profile is unsupported") is the single exception:
its exit code is `3` instead of `2` so CI can distinguish *the build cannot
handle this document* from *the document is wrong*.

## Complete code reference

| Code | Severity | Emitted by | Meaning |
|---|---|---|---|
| `NODX-E001` | fatal | `nodx-core` | Input is a ZIP package; route it through `nodx-package` first. |
| `NODX-E002` | fatal | parser, package | Input contains U+0000 or non-UTF-8 bytes. |
| `NODX-E003` | fatal | parser | Front matter started with `---` but never closed. |
| `NODX-E004` | error | validator | Front matter shape is invalid for the declared `schema`. |
| `NODX-E005` | error | parser | Block delimiters are mismatched (label, count, or no close). |
| `NODX-E006` | error | validator | Two ids in the document collide. |
| `NODX-E007` | error | validator | Cross-reference target is missing or outside its scope. |
| `NODX-E008` | error | validator | Referenced asset path does not exist in the package. |
| `NODX-E009` | error | validator | `image` block is missing the required `alt` attribute. |
| `NODX-E010` | error | package | Package contains an unsafe or invalid path. |
| `NODX-E011` | error | *(reserved)* | Reserved for include-cycle detection. The `::include` construct is a future profile (see RFC §21); baseline 1.0 processors do not emit this code. |
| `NODX-E012` | fatal/error | parser, package, url | A configured resource limit was exceeded. The message names the limit. |
| `NODX-E013` | warning | validator | Variable is referenced but never declared in front matter. |
| `NODX-E014` | error | validator | Block uses a component name with an invalid shape. |
| `NODX-E015` | warning | *(reserved)* | Reserved for a future renderer lossy-fallback signal that overlaps with `NODX-E026`. Baseline 1.0 emits `NODX-E026` instead. |
| `NODX-E016` | warning | validator | `::toc` will produce a default label because none was given. |
| `NODX-E017` | error | signing | Detached JWS signature failed verification or trust policy. |
| `NODX-E018` | fatal | parser | Source begins with a UTF-8 byte order mark. |
| `NODX-E019` | fatal | parser, package | YAML construct is outside the documented safe subset. |
| `NODX-E020` | error | validator | Link target uses an unsafe URL scheme or breaks out of the package. |
| `NODX-E021` | error | package | Package entry size or SHA-256 digest does not match the manifest. |
| `NODX-E022` | warning | validator | Heading levels skip more than one level. |
| `NODX-E023` | warning | validator | Optional profile is declared but unsupported. |
| `NODX-E024` | error | validator | Required profile is unsupported (exit code `3`). |
| `NODX-E025` | error | validator | Pipe table grid has inconsistent column counts. |
| `NODX-E026` | warning | renderer | A capability used by the document is not supported in the active renderer. |
| `NODX-E027` | error | style | NODS stylesheet contains a forbidden construct; the rule is dropped. |
| `NODX-E028` | error | validator | Front matter `integrity` is malformed or does not match the canonical AST digest. |

> `NODX-E011` and `NODX-E015` are reserved in baseline 1.0 and intentionally
> absent from the emitter map. Future profiles (`include`, renderer
> lossy-fallback split) will activate them. The codes themselves are part of
> the stable registry — do not reuse them for unrelated purposes.

## Examples

**E006 — duplicate id**

```nodx
# Intro #intro
## Intro again #intro    ← second use raises NODX-E006
```

**E022 — heading jump**

```nodx
# Title
### Sub-sub (skipped level 2)    ← raises NODX-E022 (warning)
```

**E020 — unsafe link**

```nodx
[click](javascript:alert(1))    ← raises NODX-E020, link is blocked
```

**E024 — unsupported required profile**

```nodx
---
profiles:
  requires:
    - exotic-format-v9
---
```

The validator emits `NODX-E024` and the CLI exits with status `3`. Tools
should not attempt to render documents that fail required-profile
validation.

## JSON shape

Every diagnostic serializes the same way:

```json
{
  "code": "NODX-E024",
  "severity": "error",
  "message": "Required profile `exotic-format-v9` is unsupported.",
  "line": null,
  "column": null,
  "target": "profile:exotic-format-v9"
}
```

`line` and `column` are 1-based when known, `null` for whole-document
diagnostics. `target` is a stable string identifier for the offending
artifact — an id, a path, a profile name. It is the right key to use when
deduplicating diagnostics in a CI annotation system.

## How to silence a diagnostic

You don't. Diagnostics describe facts about the document, not stylistic
preferences. If a diagnostic is wrong for your situation, the fix is one
of:

1. Change the document so it is not wrong.
2. Pass different `ResourceLimits` if you are hitting a configurable cap
   for a legitimate reason.
3. Add the missing profile to your reader, if the diagnostic is `E024` and
   you actually support the capability.

There is no `// nodx-ignore: …` escape hatch and there will not be one. The
diagnostic surface is the contract.

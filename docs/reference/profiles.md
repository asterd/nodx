# Profiles

Profiles declare what a NODX document needs from its reader. They are the
contract between authors who want a feature and tools that may or may not
implement it.

## Why profiles exist

NODX 1.0 is intentionally larger than "Markdown plus a few callouts" but
smaller than "a full publishing system". The format is large enough that a
*minimum useful reader* (think: a CI linter, a terminal viewer, a static
search indexer) cannot reasonably ship every feature. Profiles give those
readers a way to say "I handle `plain` and `core` only" without bluffing.

For authors, profiles answer: *will this document render usefully somewhere
besides my laptop?*

## The six built-in profiles

| Profile | Adds | When you need it |
|---|---|---|
| `plain` | Paragraphs, headings (1–6), inline emphasis/code, ordered/unordered lists. | Plain prose. README files. Notes. |
| `core` | Everything in `plain` plus delimited blocks (`note`, `section`, `quote`, …), links with attributes, pipe tables, `::toc`, variables. | The default for everyday docs. |
| `rich` | Figures, captions, images, footnotes, citations, mentions, forms, math, custom components. | Reports, articles, technical docs. |
| `style` | Document-level `::style` blocks and the safe NODS subset, `theme:` front matter. | Branded documents, print layouts. |
| `package` | The document expects a `.nodx` ZIP container with assets, components, or themes. | Anything that ships its own resources. |
| `agent-read` | The document is intended to be consumed via the NCP projection. | Knowledge bases, agent ingestion pipelines. |

## How a document declares profiles

```nodx
---
profiles:
  requires:
    - core
    - rich
  optional:
    - style
    - agent-read
---
```

- `requires:` lists profiles the reader must support. A reader that does
  not implement one of these stops with `NODX-E024` and exit code `3`.
- `optional:` lists profiles the document benefits from but can degrade
  without. Missing optional profiles raise `NODX-E023` warnings.

If a document omits `profiles:` entirely, the parser injects
`requires: [core]`. This is the default contract for "I have no special
needs".

## How a reader advertises support

The reference implementation supports all six built-in profiles. A
third-party reader should:

1. Declare its profile set publicly (README, integration manifest, etc.).
2. Reject required profiles it does not implement with `NODX-E024`.
3. Pass through optional-profile content as best-effort; emit `NODX-E023`
   only when behavior visibly differs.

The `Validator` type in `nodx-validate` takes a `ProfileSet` so this is
configurable from the host application.

## Forward compatibility

New profiles can be added in future revisions of the spec. The agreement
is:

- Existing profile names never change meaning.
- A reader that supports profile X across versions sees no break.
- Documents that depend on a new profile fail closed on older readers,
  which is the point — silent feature drop is the failure mode we are
  trying to avoid.

If you need a domain-specific extension, do not invent your own profile
name. Use the existing `agent-read` or `package` profile and register your
extension via component templates and front matter. Profiles are for
*capabilities the reader must implement*, not for content typing.

## Anti-patterns

- Declaring `requires: [plain]` for a document that uses tables. Wrong, it
  hides the table requirement; declare `core` instead.
- Declaring `optional: [package]` for a non-packaged document. Optional
  profiles only signal "I make use of this if you have it", not "I might
  use this someday".
- Inventing names like `requires: [my-extension]`. Use front matter for
  custom data and the package profile for custom assets.

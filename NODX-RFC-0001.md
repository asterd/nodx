# NODX-RFC-0001: NODX 1.0

**Title:** NODX 1.0 - Node-Oriented Document eXchange  
**Document type:** Final technical specification  
**Version:** 1.0  
**Date:** 11 May 2026  
**Status:** Final 1.0 reference contract  
**Primary extension:** `.nodx`  
**Text media type:** `text/nodx` (provisional)  
**Package media type:** `application/nodx+zip` (provisional)  
**Container model:** UTF-8 text or ZIP package, detected from bytes

---

## Abstract

NODX is a UTF-8, text-first, node-oriented document format for portable source
documents. A `.nodx` file can be a plain text stream or a ZIP package containing
a primary NODX document plus local assets. The format is designed for
deterministic parsing, safe rendering, explicit profile support, canonical JSON
output, stable semantic projections, and agent-readable documents.

NODX 1.0 defines the text syntax, front matter safe subset, attributes,
document tree, canonical Semantic AST, URL policy, safe package reader, NCP
semantic projection, diagnostics, command-line behavior, security baseline,
accessibility requirements, and conformance package. Features that are useful
but not required for the frozen 1.0 contract are reserved as future profiles.

---

## 1. Status of This Document

This document is the normative NODX 1.0 specification. It is self-contained:
implementers do not need any prior draft, correction note, or planning document
to implement the 1.0 format.

The format is stable at the following surfaces:

1. `schema: nodx/1.0` source documents.
2. Plain text and ZIP package detection.
3. NODX front matter safe subset.
4. Block, inline, attribute, table, literal, and custom component syntax.
5. Canonical Semantic AST JSON shape and serialization order.
6. NCP semantic projection schema `nodx-ncp/1.0` and semantic text projection rules.
7. Safe URL, asset, style, include, and package path policy.
8. NODX package manifest schema `nodx-package/1.0`.
9. Resource limit names and default ceilings.
10. Diagnostic JSON shape and error code registry.
11. CLI commands and exit code semantics.
12. Conformance package layout under `spec/conformance/v1.0`.

Any incompatible change to those surfaces requires a future major version.

The following areas are intentionally not part of mandatory NODX 1.0
conformance:

1. Agent mutation/change-record semantics.
2. Digital signature verification and trust policy.
3. Lossless concrete syntax tree editing.
4. Full computed style cascade.
5. Native PDF, DOCX, PPTX, or ODT generation.
6. Networked resource fetching.
7. Collaborative synchronization protocols.

Those capabilities are reserved as optional or future profiles so processors can
reject unsupported required capabilities deterministically.

---

## 2. Conformance and Terminology

The key words MUST, MUST NOT, REQUIRED, SHALL, SHALL NOT, SHOULD, SHOULD NOT,
RECOMMENDED, MAY, and OPTIONAL in this document are to be interpreted as
described in BCP 14, RFC 2119, and RFC 8174 when, and only when, they appear in
all capitals.

Normative sections define required processor behavior. Examples, rationale,
implementation status notes, and appendices are informative unless explicitly
marked as normative.

### 2.1 Document Classes

NODX 1.0 defines these document classes:

| Class | Description |
|---|---|
| Text NODX | A `.nodx` byte stream that is valid UTF-8 and does not start with ZIP local file header bytes. |
| Packaged NODX | A `.nodx` byte stream whose first bytes are `PK\x03\x04` and whose ZIP package follows Section 20. |
| Source document | The NODX text stream, either read directly or loaded from the package entry. |
| Semantic AST | The normalized document tree used for validation, rendering, projection, signing inputs, and conversion. |
| Canonical AST JSON | Deterministic JSON serialization of the Semantic AST. |
| NCP | NODX Compact Projection, a read-only semantic JSON projection for agents and indexing. |
| Processor | A parser, validator, renderer, converter, package reader, projection tool, CLI, editor integration, or SDK. |

### 2.2 Processor Conformance

A conforming processor MUST:

1. declare the NODX version and profile set it supports;
2. validate input against the security and resource-limit rules for its claimed
   profile;
3. report unsupported required profiles with `NODX-E024`;
4. report unsupported optional profiles with `NODX-E023`;
5. preserve document order and known semantic fields in the Canonical AST;
6. never execute document content;
7. never fetch remote resources unless host policy explicitly enables that
   behavior outside baseline NODX 1.0;
8. produce deterministic diagnostics for deterministic input.

A processor MAY implement only a subset profile, such as a parser without a
renderer. It MUST NOT claim support for a profile unless it implements that
profile's required behavior.

### 2.3 Conformance Profiles

Profile names in source front matter use short names. Profile names in prose may
use the full profile identifier.

| Short name | Full identifier | 1.0 status | Scope |
|---|---|---:|---|
| `plain` | `NODX-Plain-1.0` | stable | UTF-8 text, paragraphs, headings, safe text output. |
| `core` | `NODX-Core-1.0` | stable | Front matter, blocks, attributes, lists, core inlines, diagnostics. |
| `rich` | `NODX-Rich-1.0` | stable | Tables, figures, images, math text, footnotes, citations, forms, TOC, media fallbacks. |
| `style` | `NODX-Style-1.0` | safe subset | Inline `style` blocks and safe NODS audit/sanitization. |
| `package` | `NODX-Package-1.0` | stable reader | ZIP package opening, manifest validation, assets, digest checks. |
| `agent-read` | `NODX-Agent-Read-1.0` | stable | NCP semantic projection, semantic text projection, and stable node hashes. |
| `agent-mutate` | `NODX-Agent-Mutate-1.1` | reserved | Validated mutation records and patch application. |
| `signature` | `NODX-Signature-1.1` | reserved | JWS signatures, manifest signing, trust hooks. |
| `editor` | `NODX-Editor-1.2` | reserved | Lossless CST, source maps, local rewrites. |
| `presentation` | `NODX-Presentation-1.2` | reserved | Slide semantics and presentation export. |

Reserved profiles MAY appear in `profiles.optional`. A 1.0 processor that does
not implement them MUST warn with `NODX-E023`. If a reserved profile appears in
`profiles.requires`, a processor that does not implement it MUST report
`NODX-E024` and, for CLI use, exit with code `3`.

---

## 3. Versioning and Identification

### 3.1 Source Schema

The only normative source schema for this version is:

```yaml
schema: nodx/1.0
```

If front matter is omitted, a parser MAY still create a document with implicit
metadata equivalent to:

```yaml
schema: nodx/1.0
type: document
dir: auto
language: und
profiles:
  requires:
    - core
```

A validator that sees an explicit non-1.0 schema MUST report `NODX-E004`.

### 3.2 File Extension and Byte Detection

The primary file extension is `.nodx`.

Processors MUST NOT decide text vs package mode from the extension alone.
Detection is byte based:

1. bytes beginning with `PK\x03\x04` are Packaged NODX and MUST be passed to a
   package reader before source parsing;
2. all other bytes are Text NODX and MUST be valid UTF-8.

### 3.3 Media Types

Until formal registration is complete, implementations SHOULD use:

| Representation | Media type |
|---|---|
| Text NODX | `text/nodx; charset=utf-8` |
| Packaged NODX | `application/nodx+zip` |
| NCP semantic projection | `application/nodx-ncp+json` |
| Semantic text projection | `text/nodx-semantic; charset=utf-8` |
| NODS style sheet | `text/nodx-style` |

Formal media-type registration is out of scope for this RFC, but future
registrations SHOULD keep these names unless standards review finds a conflict.

### 3.4 Compatibility

NODX 1.0 processors MUST reject unsupported major versions. Processors MAY
accept future compatible minor metadata only if the document still declares
`schema: nodx/1.0` and no unsupported required profile is present.

---

## 4. Processing Model

A baseline processor follows this order:

1. Apply source byte limit.
2. Detect package vs text by bytes.
3. For packages, open and validate package structure, then read the entry
   source bytes.
4. Validate UTF-8.
5. Reject BOM and U+0000.
6. Normalize CRLF to LF for parsing.
7. Parse safe front matter if present.
8. Parse block and inline syntax into the Semantic AST.
9. Apply semantic validation and URL/resource policy.
10. Resolve declarative navigation.
11. Produce Canonical AST, diagnostics, renderer output, or NCP projection.

Processors MUST NOT execute scripts, macros, active media, HTML fragments,
CSS-derived scripts, package special files, symlinks, or nested ZIP archives.

Processors SHOULD continue collecting recoverable diagnostics after a non-fatal
error. Fatal diagnostics MAY stop processing if continuing would be unsafe or
would require excessive resources.

---

## 5. Resource Limits

The following defaults are part of the NODX 1.0 safety contract. Hosts MAY use
stricter limits. Hosts SHOULD NOT use looser limits for untrusted input without
an explicit risk review.

| Parameter | Default | Diagnostic |
|---|---:|---|
| Source bytes, single Text NODX | 64 MiB | `NODX-E012` fatal |
| Front matter bytes | 64 KiB | `NODX-E012` fatal |
| Line length | 1 MiB | `NODX-E012` error |
| Attribute value bytes | 64 KiB | `NODX-E012` error |
| ID bytes | 256 | `NODX-E004` or `NODX-E012` |
| Block nesting depth | 32 | `NODX-E012` error |
| Inline nesting depth | 32 | `NODX-E012` error |
| Nodes per document | 100,000 | `NODX-E012` fatal |
| Data URI bytes | 5 MiB | `NODX-E012` error |
| Expanded AST memory estimate | 64 MiB | `NODX-E012` fatal |
| Include depth | 8 | `NODX-E011` or `NODX-E012` |
| Package uncompressed bytes | 256 MiB | `NODX-E012` fatal |
| Package file count | 1,024 | `NODX-E012` error |
| Package entry bytes | 64 MiB | `NODX-E012` error |
| Package compression ratio per entry | 100:1 | `NODX-E012` fatal |
| Nested ZIP depth | 0 | `NODX-E010` fatal |
| Package path bytes | 512 | `NODX-E010` or `NODX-E012` |
| Package path segments | 8 | `NODX-E010` or `NODX-E012` |
| URL bytes | 4 KiB | `NODX-E020` error |
| Manifest entries | 1,024 | `NODX-E012` error |
| Signature header bytes | 8 KiB | reserved |
| Export bytes | 256 MiB | reserved |
| Export entry count | 1,024 | reserved |

---

## 6. Source Text Model

### 6.1 Encoding

Text NODX MUST be valid UTF-8. Invalid UTF-8 produces `NODX-E001` with severity
`fatal`.

A UTF-8 Byte Order Mark MUST be rejected by strict processors and produces
`NODX-E018` with severity `fatal`.

U+0000 MUST be rejected and produces `NODX-E002` with severity `fatal`.

### 6.2 Lines

Input MAY use LF or CRLF. Parsers normalize CRLF to LF before parsing.
Canonical JSON string values preserve semantic newlines as `\n`.

### 6.3 Parsing Scope

Block constructs are recognized at the beginning of a source line. Indented
content inside list items is treated as continued item text by the baseline
parser. Literal block content is opaque text until its matching close fence.

---

## 7. Front Matter Safe Subset

Front matter, when present, begins with a line containing exactly `---` and ends
with the next line containing exactly `---`.

```nodx
---
schema: nodx/1.0
type: document
title: Example
profiles:
  requires:
    - core
    - rich
  optional:
    - style
---
```

An unclosed front matter block produces `NODX-E003` with severity `fatal`.

### 7.1 Allowed Data Model

Front matter uses a safe YAML subset whose data model maps to:

1. null;
2. booleans `true` and `false`;
3. finite decimal numbers;
4. strings;
5. block-style lists;
6. block-style mappings;
7. literal or folded block scalar text.

The parser may accept simple scalar values in sequence items and mapping values.
Canonical AST output serializes parsed metadata as deterministic JSON.

### 7.2 Forbidden YAML Constructs

The following constructs MUST produce `NODX-E019`:

1. anchors and aliases;
2. explicit tags;
3. merge keys;
4. duplicate keys;
5. multiple YAML documents;
6. tab indentation;
7. flow-style collections;
8. native timestamps;
9. non-finite numbers;
10. hexadecimal, binary, or octal numeric specials;
11. YAML boolean aliases such as `yes`, `no`, `on`, and `off`;
12. custom objects or executable tags;
13. control characters.

### 7.3 Standard Metadata Fields

| Field | Type | Requirement |
|---|---|---|
| `schema` | string | MUST be `nodx/1.0` when explicit. |
| `type` | string | SHOULD be `document` for normal documents. |
| `title` | string | RECOMMENDED for Core, REQUIRED by many Rich/rendering workflows. |
| `language` | string | BCP 47 language tag or `und`; default `und`. |
| `dir` | string | `ltr`, `rtl`, or `auto`; default `auto`. |
| `profiles.requires` | list of strings | Required profile short names. |
| `profiles.optional` | list of strings | Optional profile short names. |
| `vars` | mapping | Named variables referenced as `{{vars.name}}`. |
| `components` | list of mappings | Custom component declarations. |
| `keywords` | list of strings | Informative indexing metadata. |
| `theme` | string | Optional standard theme name or package-local `.nodt` path. |

Processors MUST preserve unknown metadata fields in the Canonical AST unless the
field itself violates the front matter safe subset.

### 7.4 Profiles

Profile declarations use block-style lists for maximum interoperability:

```yaml
profiles:
  requires:
    - core
    - rich
  optional:
    - style
```

Unsupported required profiles produce `NODX-E024` with severity `error`.
Unsupported optional profiles produce `NODX-E023` with severity `warning`.

### 7.5 Themes

The optional `theme` field selects a safe default style package for renderers:

```yaml
theme: web
```

NODX 1.0 reserves these standard theme names:

| Theme | Purpose |
|---|---|
| `none` | No visual styling beyond semantic renderer minimum. |
| `base` | Neutral readable typography. |
| `web` | Responsive browser-oriented defaults. |
| `print` | Print/PDF-oriented defaults, including page margins and page breaks. |
| `presentation` | Large-type defaults for slide-like previews. |

A package-local `.nodt` path MAY be used by renderers that implement the theme
format. Theme files are declarative resources, not executable code. Remote theme
fetching is outside baseline NODX 1.0.

---

## 8. Attributes

Blocks, headings, and spans MAY carry attributes in braces:

```nodx
# Title {#intro .lead role="doc-introduction"}
# Short title #intro

:::note {#risk .warning fallback="children"}
Fallback content.
:::

[Important text]{.mark data-tone="urgent"}
```

Attribute syntax consists of space-separated tokens:

| Token | Meaning |
|---|---|
| `#id` | Node or inline span identifier. |
| `.class` | CSS-like class. Multiple classes are allowed. |
| `key="value"` | String attribute. |
| `key=value` | Unquoted string attribute. |

Headings MAY use a light ID form at the end of the heading line:

```nodx
# Introduction #intro
```

The light form is valid only on headings, only at the end of the line, and only
for a single ID token matching `#` followed by an ASCII identifier. Classes and
key/value attributes still require the braced form. A literal trailing token can
be escaped, for example `# Introduction \#intro`.

IDs and names MUST begin with an ASCII letter and then contain ASCII letters,
ASCII digits, and, where allowed, hyphens. Duplicate classes are deduplicated
and class output is sorted in canonical form. Attribute maps are sorted by key
in Canonical AST JSON.

The common `dir` attribute, when present, MUST be one of `ltr`, `rtl`, or
`auto`.

---

## 9. Semantic AST

### 9.1 Document Object

The Semantic AST root contains:

| Field | Type | Meaning |
|---|---|---|
| `schema` | string | Always `nodx/1.0` for this version. |
| `meta` | object | Parsed front matter plus implicit defaults. |
| `body` | array | Ordered top-level nodes. |

Parser diagnostics are not part of Canonical AST JSON.

### 9.2 Node Object

Every node has these canonical fields in this order:

| Field | Type | Meaning |
|---|---|---|
| `attrs` | object | String attributes sorted by key. |
| `children` | array | Ordered child nodes. |
| `classes` | array | Sorted class names. |
| `id` | string or null | Stable node identifier. |
| `inlines` | array | Inline content for textual nodes. |
| `text` | string or null | Literal text for literal nodes. |
| `type` | string | Node type name. |

Exactly one of `children`, `inlines`, or `text` is normally non-empty, but the
canonical shape always emits all fields.

### 9.3 Canonical JSON

Canonical AST JSON MUST:

1. use UTF-8 JSON text;
2. omit insignificant whitespace;
3. sort object keys in metadata and attribute maps;
4. serialize node fields in the fixed order from Section 9.2;
5. serialize document fields in the order `body`, `meta`, `schema`;
6. escape JSON strings with standard JSON escapes;
7. preserve source order for arrays;
8. exclude concrete syntax trivia, diagnostics, source byte ranges, renderer
   computed style, and editor state.

Canonical AST is the stable semantic contract for independent parsers.

---

## 10. Block Syntax

### 10.1 Paragraphs

Consecutive non-empty lines that are not another block construct form a
paragraph. The paragraph node has `type: "paragraph"` and parsed inline content.

### 10.2 Headings

Headings use one to six `#` characters followed by one space:

```nodx
# Level 1 {#intro}
## Level 2
```

Heading nodes have `type: "heading"` and an attribute `level` whose value is
`"1"` through `"6"`. Invalid heading levels produce `NODX-E004`. Heading level
jumps SHOULD produce `NODX-E022`.

### 10.3 Lists

Unordered list items begin with `- `. Ordered list items begin with an ASCII
decimal number followed by `. `. Task items begin with `- [ ] ` or `- [x] `.

Canonical nodes:

1. list container: `type: "list"`, `attrs.kind` equal to `unordered`,
   `ordered`, or `task`;
2. item: `type: "item"`;
3. task item checked state: `attrs.checked` equal to `true` or `false`.

Baseline NODX 1.0 list parsing is flat. Continued lines indented by two spaces
are appended to the item text.

### 10.4 Pipe Tables

Pipe table syntax:

```nodx
| Name | Value |
| - | - |
| AST | canonical |
```

Markdown-compatible separator rows such as `|---|---|` and alignment markers
such as `|---:|:---|` are accepted and map to the same table AST. Alignment
markers are authoring sugar unless a renderer/profile explicitly consumes them.

Canonical nodes:

1. `table`;
2. child `row`;
3. child `cell`;
4. header row cells carry `header="true"` and `scope="col"`.

All table rows MUST have the same number of cells. Violations produce
`NODX-E025`.

### 10.5 Delimited Blocks

Delimited blocks use two or more colons, a node name, optional attributes, and a
matching close fence with the same colon count:

```nodx
::section {#overview}
## Overview

Text inside the section.
::
```

A close fence MAY carry a label:

```nodx
::figure {#pipeline}
::image {src="assets/pipeline.png" alt="Pipeline diagram"}
::
::caption
Pipeline diagram.
::
:: figure
```

The close label, when present, MUST match the opening node name. Unmatched,
mismatched, or unclosed blocks produce `NODX-E005`.

For compatibility with the reference corpus, processors MAY also accept the
compact contextual close form where the name immediately follows the colon run
and matches the currently open block, such as `::figure`.

Node names MUST begin with an ASCII letter and may contain ASCII letters,
digits, and hyphens.

The three-colon form remains valid. Authors MAY use three or more colons as an
escape when literal content or deep nesting would make the two-colon form less
readable.

### 10.6 Literal Blocks

The block names `code`, `pre`, `math`, and `style` are literal blocks. Their
source content is preserved as `text` and is not parsed as child blocks or
inlines.

```nodx
::code {lang="rust"}
fn main() {}
::
```

---

## 11. Standard Block Nodes

NODX parsers preserve any syntactically valid delimited block name. Validators
and renderers assign normative meaning to the following standard nodes.

| Node | Profile | Shape | Meaning |
|---|---|---|---|
| `paragraph` | plain/core | inlines | Paragraph text. |
| `heading` | plain/core | inlines | Section heading with `level`. |
| `list` | core | children | Ordered, unordered, or task list. |
| `item` | core | inlines | List item. |
| `section` | core/rich | children | Semantic grouping. |
| `note` | rich | children | Note, callout, aside, warning, or fallback container. |
| `quote` | rich | children | Quoted block. |
| `code` | core/rich | text | Literal code or preformatted text. |
| `pre` | core/rich | text | Preformatted text. |
| `math` | rich | text | Literal math source. |
| `table` | rich | children | Table container. |
| `row` | rich | children | Table row. |
| `cell` | rich | inlines/children | Table cell. |
| `figure` | rich | children | Figure container. |
| `caption` | rich | inlines/children | Figure/table caption. |
| `image` | rich | attrs/children | Static image reference. |
| `media` | rich | attrs/children | Local media reference with fallback. |
| `embed` | rich | attrs/children | Local embeddable resource with fallback. |
| `media-fallback` | rich | children | Fallback content for media/embed. |
| `include` | package/rich | attrs | Package-local include reference. |
| `toc` | rich | attrs | Declarative navigation node. |
| `style` | style | text | Safe NODS stylesheet block. |
| `form` | rich | children | Read-only or declarative form grouping. |
| `field` | rich | attrs/children | Field metadata and visible fallback. |
| `pagebreak` | rich | attrs | Paged-output break hint. |
| `bibliography` | rich | children | Bibliography container. |
| `citation-entry` | rich | children | Citation metadata/fallback entry. |
| `speaker-notes` | presentation reserved | children | Preserved fallback notes; presentation semantics are deferred. |
| `slide` | presentation reserved | children | Preserved custom block unless presentation profile is implemented. |

Custom component names containing hyphens are allowed. If a custom component is
not declared in front matter and lacks an explicit `fallback` attribute, a
validator SHOULD emit `NODX-E014` with severity `warning`.

Unknown blocks MUST preserve children in the AST. Renderers SHOULD render
fallback children when they cannot render the block's specialized semantics.

---

## 12. Inline Syntax

Inline parsing applies inside paragraphs, headings, list items, table cells, and
other textual nodes.

| Source | Inline type | Canonical fields |
|---|---|---|
| plain text | `text` | `text` |
| `` `code` `` | `code` | `text` |
| `**strong**` | `strong` | `children` |
| `*emphasis*` | `em` | `children` |
| `==mark==` | `mark` | `children` |
| `~sub~` | `sub` | `children` |
| `^sup^` | `sup` | `children` |
| `[label](target)` | `link` | `label`, `target` |
| `[label](target){attrs}` | `link` | `label`, `target`, `attrs` |
| `[label]{attrs}` | `span` | `children`, `attrs` |
| `{{namespace.name}}` | `var` | `namespace`, `name` |
| `{{name}}` | `var` | `namespace: "vars"`, `name` |
| `@[target]` | `ref` | `target` |
| `@{kind:target}` | `mention` | `kind`, `target` |
| `[^target]` | `footnote-ref` | `target` |
| `[@target]` | `citation-ref` | `target` |
| `$$source$$` | `math-inline` | `source` |

Backslash escapes the following characters in inline text:

```text
` * [ ] ( ) { } # @ ~ ^ = : |
```

Inline links MUST pass the URL policy in Section 19. Link attributes use the
same attribute grammar as spans. Portable renderers SHOULD support `title`,
`rel`, and `download`; unsupported link attributes MUST be preserved in the
Semantic AST and ignored safely by renderers that cannot use them. References,
footnote references, and citation references are validated against known node
IDs. Unresolved references produce `NODX-E007`.

Variables without an explicit namespace are canonicalized to the `vars`
namespace. Variables in the `vars` namespace SHOULD be declared in front matter.
An undeclared `{{vars.name}}` or `{{name}}` reference produces `NODX-E013` with
severity `warning`.

---

## 13. Declarative Navigation and `toc`

`toc` is a Rich block node with no required source content. It declares a
navigation projection; generated entries are not inserted into the source AST.

```nodx
:::toc {#main role="primary" source="document" depth="2" title="Contents"}
:::

:::section {#chapter}
## Chapter
:::toc {role="local" scope="#chapter" min-level="2" max-level="4"}
:::
:::
```

Attributes:

| Attribute | Values | Default |
|---|---|---|
| `role` | `primary`, `local`, `secondary`, `breadcrumb` | `primary` |
| `source` | `document` | `document` |
| `scope` | `#id` | whole document |
| `depth` | integer `1` through `6` | `6` |
| `min-level` | integer `1` through `6` | first heading level in scope |
| `max-level` | integer `1` through `6` | `min-level + depth - 1`, capped at 6 |
| `title` | string | deterministic role label |
| `mode` | `auto`, `manual` | `auto` |

Invalid attributes produce `NODX-E004`. An unresolved `scope` produces
`NODX-E007`. If `title` is omitted, processors use a deterministic accessible
label and MAY emit `NODX-E016` with severity `info`.

Resolved `toc` entries contain heading ID, heading level, plain-text title, and
node path. NCP exposes resolved entries as `navigationEntries` on `toc` nodes.

Heading attributes MAY tune generated TOC rows:

```nodx
# Chapter 1: System architecture {#arch toc="Architecture"}
## Internal implementation detail {#impl toc-hidden="true"}
```

`toc` overrides the generated row title, `toc-hidden="true"` excludes the
heading, and `toc-level` MAY override the navigation level without changing the
visible heading level.

A manual TOC uses normal list/link content:

```nodx
::toc {title="Recommended path" mode="manual"}
- [Overview](#overview)
- [API essentials](#api)
::
```

---

## 14. Images, Media, and Embeds

### 14.1 Image

An image node references a safe asset:

```nodx
:::image {src="assets/photo.png" alt="Product photo"}
:::
```

Rules:

1. `src` MUST pass the asset URL policy.
2. Informative images MUST have non-empty `alt`.
3. Decorative images MUST use `decorative="true"` and MAY omit `alt`.
4. Missing `alt` on an informative image produces `NODX-E009`.
5. Unsafe or unresolvable assets produce `NODX-E008`.

### 14.2 Figure and Caption

`figure` is a container for image/media/embed/table content. `caption` is a
human-visible description. Renderers SHOULD map those nodes to native semantic
containers when available.

### 14.3 Media and Embed

`media` and `embed` reference package-local resources. Renderers that cannot
display the resource MUST render fallback children if present.

```nodx
:::media {src="media/demo.mp4" alt="Demo video"}
:::media-fallback
Demo transcript.
:::
:::
```

Remote media fetching is not part of NODX 1.0.

---

## 15. Forms and Read-Only Fields

NODX Rich includes declarative form and field nodes for structured display,
review checklists, and agent-readable data. NODX 1.0 does not define active
form submission.

```nodx
:::form {#approval}
:::field {name="owner" label="Owner" value="Legal"}
:::
:::
```

Renderers SHOULD expose labels and values accessibly. Processors MUST NOT treat
form fields as executable controls unless a host application explicitly maps
them to trusted UI outside baseline NODX processing.

---

## 16. Math

Block math uses the literal `math` block. Inline math uses `$$...$$`.

```nodx
:::math
E = mc^2
:::

Inline math: $$a^2 + b^2 = c^2$$.
```

NODX 1.0 stores math source text and does not mandate a math rendering engine.
Renderers that support math MUST escape or sanitize renderer output for the
target context.

---

## 17. Bibliography, Footnotes, and Citations

NODX 1.0 reserves semantic nodes for bibliography and citation content and
inline references for footnotes and citations:

```nodx
This claim is cited [@doe2026] and has a note [^n1].

:::bibliography
:::citation-entry {#doe2026}
Doe, Example Study, 2026.
:::
:::

:::note {#n1}
Footnote text.
:::
```

Processors validate `footnote-ref` and `citation-ref` targets against known
node IDs. Bibliographic style formatting is outside the baseline 1.0 contract.

---

## 18. NODS Style Profile

The Style profile defines a safe subset for literal `style` blocks. Full CSS
cascade, layout interoperability, and computed style are not mandatory NODX 1.0
features.

```nodx
::style
h1, .lead { color: #0f766e; font-size: 24pt; }
@media print { p { color: black; } }
@page { size: A4 portrait; margin: 22mm; }
::
```

Authors and generators MAY also use YAML style authoring syntax:

```nodx
::style {format="yaml"}
h1:
  color: "#0f766e"
  font-size: 24pt
print:
  p:
    color: black
page:
  margin: 22mm
::
```

YAML style blocks are converted to the same audited NODS/CSS subset before
rendering. Top-level pseudo-keys are limited to `print`, `screen`, `dark`, and
`page`; all other top-level keys are treated as selectors. Invalid YAML style
structure produces `NODX-E027`.

### 18.1 Required Safety Behavior

A Style processor MUST audit inline `style` blocks and report forbidden
constructs as `NODX-E027`. HTML renderers MUST sanitize style text before
emitting it into a `<style>` context.

Forbidden constructs include:

1. `</style` breakout text, including CSS-escaped variants;
2. `<script`, `<svg`, `<iframe`, `<object`, and `<embed` breakout text;
3. `javascript:`, `vbscript:`, and `expression(`;
4. remote or unsafe `url(...)` values;
5. interactive pseudo-classes such as `:hover` and `:focus`;
6. pseudo-elements;
7. universal or namespace selectors;
8. executable binding properties such as `behavior`, `binding`,
   `-moz-binding`, and `-ms-behavior`.

### 18.2 Allowed Selector Shape

The safe subset accepts simple selectors over known document/rendering tags,
classes, IDs, selected non-interactive structural pseudo-classes, safe attribute
selectors, and child/sibling combinators. Attribute selectors are limited to
safe attribute names such as `lang`, `dir`, `role`, and selected `data-*`
attributes.

### 18.3 At-Rules

The safe subset accepts `@page`, `@media`, and `@supports` wrappers subject to
the same nested rule audit. Other at-rules MUST produce `NODX-E027`.

### 18.4 Properties

The safe subset accepts inert document styling properties for color,
typography, spacing, borders, table layout, print hints, grid/flex layout
properties, writing mode, and custom properties. Unsupported but non-executable
properties MAY produce `NODX-E027` with severity `warning`. Executable or
breakout properties MUST produce `NODX-E027` with severity `error`.

Style URLs use the Style reference policy: package-relative paths only.

### 18.5 Standard Design Tokens

Standard themes expose these custom properties for safe overrides:

```css
--nodx-color-text
--nodx-color-muted
--nodx-color-primary
--nodx-color-accent
--nodx-font-body
--nodx-font-heading
--nodx-font-mono
--nodx-page-margin
--nodx-line-height
--nodx-block-gap
```

Renderers MAY add additional tokens, but these names are reserved and stable.

---

## 19. URL, Asset, and Path Policy

NODX is offline by default. Processors MUST NOT fetch network resources unless a
host application explicitly enables network access outside the baseline 1.0
contract.

### 19.1 Common URI Rules

URI strings MUST:

1. be non-empty;
2. fit within the URL byte limit;
3. contain no C0 controls or DEL;
4. contain no backslash;
5. reject unsafe schemes after ASCII percent-decoding of the scheme prefix.

Forbidden schemes include `javascript`, `vbscript`, `file`, `jar`, `chrome`,
and `about`.

### 19.2 Reference-Kind Policy

| Reference kind | Allowed |
|---|---|
| Link | fragments, `http`, `https`, `mailto`, `tel`, and safe package-relative paths. |
| Asset | safe package-relative paths and allowed image data URIs. |
| Style | safe package-relative paths only. |
| Include | safe package-relative paths only. |
| Font | safe package-relative paths only. |
| Media fallback | safe package-relative paths only. |

Unsafe link targets produce `NODX-E020`. Unsafe assets produce `NODX-E008`.
Unsafe package paths produce `NODX-E010`.

### 19.3 Package-Relative Paths

Package-relative paths:

1. MUST NOT be absolute;
2. MUST NOT contain a scheme prefix;
3. MUST NOT contain empty segments;
4. MUST NOT contain `.` or `..` segments;
5. MUST NOT contain percent-decoded `..`, slash, backslash, colon, or control
   characters inside a segment;
6. MUST use `/` as separator;
7. MUST fit the path byte and segment limits.

### 19.4 Data URIs

Data URIs are allowed only for Asset references and only for these media types:

1. `image/png`;
2. `image/jpeg`;
3. `image/webp`;
4. `image/gif`.

SVG data URIs are not part of the NODX 1.0 safe baseline.

---

## 20. Packaged NODX

Packaged NODX is a ZIP archive stored in a `.nodx` file. A package reader is a
read-only virtual file system. It MUST NOT extract entries to disk as part of
baseline parsing.

### 20.1 Required Package Structure

A minimal package contains:

```text
mimetype
manifest.yaml
doc.nodx
```

The first ZIP entry MUST be named exactly `mimetype`, MUST be stored
uncompressed, and MUST contain exactly:

```text
application/nodx+zip
```

The package manifest MUST be named `manifest.yaml`.

### 20.2 ZIP Requirements

A package reader MUST reject:

1. empty packages;
2. multi-disk ZIP files;
3. ZIP64 packages;
4. encrypted entries;
5. data descriptors;
6. unsupported compression methods;
7. compression methods other than stored (`0`) and DEFLATE (`8`);
8. entries whose local headers differ from the central directory;
9. duplicate normalized entry paths;
10. non-UTF-8 entry names;
11. symlinks, device files, directories masquerading as files, and other
    special files;
12. nested ZIP entries;
13. CRC mismatches;
14. package paths that violate Section 19.3;
15. packages that exceed Section 5 limits.

### 20.3 Manifest Schema

Manifest schema is:

```yaml
schema: nodx-package/1.0
entry: doc.nodx
profiles:
  requires:
    - core
    - rich
  optional:
    - style
entries:
  - path: doc.nodx
    size: 1234
    sha256: sha256-BASE64URLDIGEST
signature: signatures/package.jws
```

Fields:

| Field | Type | Requirement |
|---|---|---|
| `schema` | string | MUST be `nodx-package/1.0`. |
| `entry` | path | REQUIRED primary `.nodx` entry. |
| `profiles.requires` | list | Optional package-required profile set. |
| `profiles.optional` | list | Optional package-optional profile set. |
| `entries` | list | SHOULD list package files with size/digest. |
| `entries[].path` | path | REQUIRED for each listed entry. |
| `entries[].size` | integer | OPTIONAL exact byte length. |
| `entries[].sha256` | string | OPTIONAL `sha256-` base64url digest. |
| `signature` | path | OPTIONAL reserved signature path. |

Manifest parsing uses the same safe-subset principles as front matter and
rejects anchors, aliases, tags, merge keys, duplicate top-level keys, flow-style
YAML, inline comments, and unknown manifest fields.

If a listed manifest entry is missing, if a size does not match, or if a digest
does not match, the package reader MUST report `NODX-E021`.

The `signature` field is validated as a package path and as an existing file if
present. Signature verification semantics are reserved for the Signature
profile.

---

## 21. Includes

The `include` node references package-local NODX content:

```nodx
:::include {src="partials/appendix.nodx"}
:::
```

Rules:

1. `src` MUST be a safe package-relative path.
2. Remote includes are forbidden.
3. Include expansion is optional for baseline processors.
4. A processor that expands includes MUST enforce include depth.
5. Include cycles MUST produce `NODX-E011`.
6. Included IDs participate in the resolved document ID namespace unless a
   future include profile defines prefixing semantics.

Processors that do not implement include expansion MUST still validate the
`src` attribute and preserve the `include` node in the AST.

---

## 22. Agent-Readable Projections

NODX 1.0 defines two standard read-only projections for agents, indexing,
search, review, and context generation:

1. NCP, a deterministic JSON tree projection for tools that need addresses,
   hashes, attributes, navigation entries, and stable node identity.
2. Semantic text, a compact plain-text projection for LLM prompt context and
   human review where token cost matters more than machine-addressable fields.

Neither projection is the authoritative source when original NODX source or the
Canonical AST is available.

### 22.1 NCP Semantic Projection

The NCP 1.0 schema is:

```json
"schema": "nodx-ncp/1.0"
```

NCP output fields:

| Field | Type | Meaning |
|---|---|---|
| `schema` | string | `nodx-ncp/1.0`. |
| `mode` | string | `semantic`. |
| `sourceHash` | string | SHA-256 base64url digest of Canonical AST JSON. |
| `nodes` | array | Top-level semantic node records; each node preserves child records recursively. |
| `chunks` | array | Deterministic chunk metadata over node IDs or path-derived IDs. |
| `loss` | array | Projection loss records. Empty means no loss inside the NCP semantic contract, not that the projection is a Canonical AST clone. |

NCP node fields:

| Field | Type | Meaning |
|---|---|---|
| `type` | string | Node type. |
| `id` | string | Node ID or empty string. |
| `path` | string | Dot-separated child index path. |
| `attrs` | object | String attributes. |
| `text` | string | Literal text or plain inline text. |
| `children` | array | Child NCP nodes. |
| `sha256` | string | Deterministic node digest. |
| `navigationEntries` | array | Present on `toc` nodes. |

Hashes use `sha256-` plus unpadded base64url digest bytes.

NCP paths are structural addresses. Authors SHOULD provide stable node IDs for
content that agents, editors, or integrations need to address across revisions.

NCP deliberately excludes concrete syntax trivia, computed styles, renderer
templates, host layout results, and custom component render output. It preserves
custom component source semantics as ordinary node records: `type`, `id`,
`path`, `attrs`, `text`, `children`, and `sha256`. Renderers or agents that do
not understand a custom component MUST treat its children as fallback source
content rather than expanding a renderer-specific template.

### 22.2 Semantic Text Projection

Semantic text is a deterministic UTF-8 text projection with media type
`text/nodx-semantic; charset=utf-8`. It is optimized for compact LLM context,
plain-text search, and quick human inspection. It is not suitable for signing,
patch addressing, or lossless interchange.

Semantic text processors MUST:

1. preserve document order for included nodes;
2. emit headings as Markdown-style `#` lines with stable IDs when present;
3. emit paragraphs, lists, tables, figures, images, captions, code, math,
   quotes, notes, forms, fields, media/embed/include fallback text,
   bibliography entries, and custom component fallback children;
4. terminate output with exactly one trailing newline;
5. never execute or expand renderer templates, component renderers, scripts,
   remote resources, or host layout output.

Semantic text processors MUST exclude:

1. `style` nodes and component style blocks;
2. `toc` nodes, because resolved navigation is represented by headings or NCP
   `navigationEntries`;
3. `pagebreak` nodes and automatic page boundaries;
4. computed CSS, theme output, HTML attributes created by renderers, and
   package manifest metadata;
5. custom component template output. The source custom component marker and its
   fallback children remain visible.

For custom components, semantic text MUST emit a component marker containing
the component name and semantic attributes, then recursively emit fallback
children. If fallback children are absent, the component marker is still emitted
so the omission is visible to consumers.

---

## 23. Diagnostics

Stable diagnostic JSON contains exactly:

| Field | Type |
|---|---|
| `code` | string |
| `severity` | string: `info`, `warning`, `error`, or `fatal` |
| `message` | string |
| `line` | integer or null |
| `column` | integer or null |
| `target` | string or null |

JSON diagnostics MUST be deterministic for deterministic input.

### 23.1 Error Registry

| Code | Default severity | Owner | Meaning |
|---|---|---|---|
| `NODX-E001` | fatal | core | Invalid UTF-8 or invalid byte-level input for parser. |
| `NODX-E002` | fatal | core | U+0000 is not allowed. |
| `NODX-E003` | fatal | core | Unclosed front matter. |
| `NODX-E004` | error | validate | Missing or invalid schema, name, heading level, TOC attr, or direction value. |
| `NODX-E005` | error | core | Malformed, unmatched, mismatched, or unclosed delimited block. |
| `NODX-E006` | error | validate | Duplicate node ID. |
| `NODX-E007` | error | validate | Unresolved reference or TOC scope. |
| `NODX-E008` | error | url/validate | Unresolvable or unsafe asset. |
| `NODX-E009` | error | validate | Informative image missing non-empty alt text. |
| `NODX-E010` | error/fatal | url/package | Unsafe package path, path traversal, nested ZIP, or unsafe package structure. |
| `NODX-E011` | error | include/package | Include cycle. |
| `NODX-E012` | error/fatal | core/package | Resource limit exceeded or malformed ZIP envelope. |
| `NODX-E013` | warning | validate | Variable referenced but not declared. |
| `NODX-E014` | warning | validate | Undeclared custom component without explicit fallback. |
| `NODX-E015` | warning | renderer | Renderer emitted lossy fallback. |
| `NODX-E016` | info/warning | validate | Recoverable default or unknown/preserved feature. |
| `NODX-E017` | error | package | Media type mismatch. |
| `NODX-E018` | fatal | core | Byte Order Mark is not allowed. |
| `NODX-E019` | fatal | front matter/package | Forbidden YAML safe-subset construct. |
| `NODX-E020` | error | url | Unsafe URL or scheme. |
| `NODX-E021` | error | package | Manifest entry, size, or digest mismatch. |
| `NODX-E022` | warning | validate | Heading level jumps over an intermediate level. |
| `NODX-E023` | warning | validate | Unsupported optional profile or optional feature. |
| `NODX-E024` | error | validate | Unsupported required profile or required feature. |
| `NODX-E025` | error | validate | Table rows have inconsistent cell counts. |
| `NODX-E026` | warning | export | Lossy export or preview bridge warning. |
| `NODX-E027` | error/warning | style | Forbidden or unsupported NODS construct. |

---

## 24. Command-Line Interface

A conforming CLI SHOULD implement:

```text
nodx inspect <file>
nodx ast <file> [--format text|json]
nodx validate <file> [--profile plain|core|rich|style|package|agent-read] [--format text|json]
nodx diagnostics <file> [--format text|json]
nodx html <file> [--standalone|--fragment] [--csp|--no-csp]
nodx tui <file>
nodx ncp <file> [--mode semantic]
nodx semantic <file>
nodx package inspect <file>
nodx package verify <file>
```

Preview or reserved commands MAY exist, but they MUST label themselves unstable
or profile-specific. For example, `nodx export pdf|docx|pptx` is not part of
mandatory NODX 1.0 export conformance.

Exit codes:

| Code | Meaning |
|---:|---|
| `0` | Success, or warnings only. |
| `1` | I/O failure, CLI usage failure, or unsupported CLI option. |
| `2` | Parse, validation, package, URL, style, or security failure. |
| `3` | Required profile or required feature unsupported by this CLI build. |

---

## 25. Rendering Requirements

A renderer MUST escape output for the target context. An HTML renderer MUST
escape text, attribute values, URLs, and style contexts separately. A renderer
MUST NOT insert document-derived strings through raw HTML APIs unless the string
was produced by an audited sanitizer for that exact context.

Renderers SHOULD:

1. use semantic host elements where available;
2. preserve source order as reading order;
3. render unknown blocks by rendering fallback children;
4. expose heading levels and navigation accessibly;
5. expose image alt text;
6. block unsafe URLs;
7. avoid loading remote resources by default;
8. include an appropriate Content Security Policy for standalone HTML output.

---

## 26. Security Considerations

NODX documents are untrusted input. A secure processor MUST:

1. enforce resource limits before allocation-heavy operations;
2. reject invalid UTF-8, BOM, and U+0000 as specified;
3. reject forbidden YAML constructs;
4. never execute embedded content;
5. never fetch remote resources by default;
6. reject unsafe URL schemes and path traversal;
7. reject ZIP special files, encrypted entries, nested ZIPs, and ZIP64;
8. validate package digests when declared;
9. sanitize NODS style blocks;
10. escape renderer output by context;
11. avoid writing package entries to disk during parsing;
12. treat signature keys and trust roots as host policy, not document truth.

Processors SHOULD warn or provide host-level safeguards for isolated
bidirectional control characters, suspicious mixed-script identifiers, and
visually confusable labels in security-sensitive contexts.

---

## 27. Accessibility and Internationalization

NODX source is Unicode text. `language` metadata and `lang` attributes use BCP
47 language tags. `dir` values are `ltr`, `rtl`, and `auto`.

Renderers SHOULD map language and direction to target-native mechanisms, such
as HTML `lang` and `dir` attributes.

Accessible output requirements:

1. informative images require `alt`;
2. decorative images use `decorative="true"`;
3. heading order should avoid skipped levels;
4. tables should expose header cells and scopes;
5. links should have descriptive labels;
6. form fields should expose labels;
7. navigation generated from `toc` should have a deterministic label;
8. source order should match reading order;
9. generated HTML should use semantic elements where practical.

---

## 28. Interoperability and Conversion

NODX is an authoritative source and exchange format, not a pixel-perfect final
presentation format. Importers and exporters SHOULD report semantic loss.

Recommended exporters for a mature ecosystem:

1. HTML;
2. Markdown;
3. plain text;
4. PDF through an audited host renderer;
5. DOCX with loss report;
6. PPTX for the future presentation profile;
7. NCP JSON.

Recommended importers:

1. Markdown to Core/Rich NODX;
2. HTML safe subset to Rich NODX;
3. DOCX/ODT structural import with loss report;
4. CSV/TSV to table nodes.

Typed table schemas and CSV/TSV import/export are ecosystem tooling
recommendations, not mandatory NODX 1.0 conformance requirements. Parquet/Arrow
portability is reserved for a future RFC.

Lossy conversion SHOULD emit machine-readable reports using stable diagnostics,
including `NODX-E026` for known export loss.

---

## 29. Conformance Package

The implementer-facing conformance package lives at:

```text
spec/conformance/v1.0
```

Its layout is:

```text
fixtures/
expected/
manifest.json
README.md
```

A conforming independent implementation SHOULD run the package and compare
Canonical AST, NCP, and diagnostics byte-for-byte for the relevant profile.
Reference HTML output is informative unless the implementation claims the same
HTML renderer profile.

The repository verification command is:

```sh
rtk sh scripts/verify_conformance_package.sh
```

---

## 30. Grammar Summary

This grammar is informative. Semantic constraints such as matching fence length,
matching close labels, safe paths, and profile validation are normative in the
sections above.

```abnf
document        = [front-matter] *(blank / block)
front-matter    = "---" LF *yaml-line "---" LF

block           = heading / delimited / list / pipe-table / paragraph
blank           = *WSP LF

heading         = 1*6"#" SP inline-text [SP attrs] LF

delimited       = opener LF *(block / literal-line) closer LF
opener          = 2*":" name [SP attrs]
closer          = same-colon-count [SP name / name]
name            = ALPHA *(ALPHA / DIGIT / "-")

list            = 1*(unordered-item / ordered-item / task-item)
unordered-item  = "- " inline-text LF
ordered-item    = 1*DIGIT ". " inline-text LF
task-item       = "- [" (" " / "x") "] " inline-text LF

pipe-table      = pipe-row LF pipe-separator LF *pipe-row
pipe-row        = ["|"] cell *("|" cell) ["|"]
pipe-separator  = ["|"] 1*("-" / ":" / SP / "|") ["|"]

paragraph       = inline-text *(LF inline-text)

attrs           = "{" [attr-token *(SP attr-token)] "}"
attr-token      = id-token / class-token / key-value
id-token        = "#" name
class-token     = "." name
key-value       = name "=" (quoted / unquoted)
```

---

## 31. Complete Example

```nodx
---
schema: nodx/1.0
type: document
title: NODX Example
language: en
dir: auto
profiles:
  requires:
    - core
    - rich
  optional:
    - style
    - agent-read
vars:
  reviewer: Ada
components:
  - name: approval-card
---

:::toc {#contents role="primary" depth="2" title="Contents"}
:::

# NODX Example {#intro}

Hello **structured** world. Reviewer: {{vars.reviewer}}.

This links to [the project](https://example.com) and references @[table].

:::style
h1 { color: #0f766e; }
.warning { border-left: 4px solid #b91c1c; padding-left: 12px; }
:::

:::note {#risk .warning fallback="children"}
Unknown renderers preserve this fallback content.
:::

| Feature | Status |
| - | - |
| Canonical AST | stable |
| NCP | stable |

:::figure {#pipeline}
:::image {src="assets/pipeline.png" alt="Pipeline diagram"}
:::
:::caption
Reference processing pipeline.
:::
:::
```

---

## 32. Implementation Status

The reference repository contains:

1. Rust crates for parsing, canonical AST, validation, URL policy, style audit,
   package reading, HTML rendering, TUI rendering, NCP projection, CLI, signing
   support, editor/CST experiments, agent mutation experiments, and export
   previews;
2. an independent JavaScript parser/projection package;
3. conformance, negative, navigation, rendering, security, package, export, and
   presentation fixtures;
4. starter editor integrations for VSCode, Sublime Text, and Notepad++;
5. showcase documents for web and TUI/desktop rendering.

The reference parser and validator include the Lite authoring forms described
above: two-colon blocks, heading light IDs, short variables, link attributes,
Markdown-compatible table separators, manual TOC entries, standard themes, and
YAML style authoring.

Only the stable behavior defined by this RFC is required for NODX 1.0
conformance. Experimental crates and preview commands do not extend the
mandatory 1.0 contract unless their behavior is explicitly described in this
document.

---

## 33. References

### 33.1 Normative References

1. RFC 2119, "Key words for use in RFCs to Indicate Requirement Levels".
2. RFC 8174, "Ambiguity of Uppercase vs Lowercase in RFC 2119 Key Words".
3. RFC 8259, "The JavaScript Object Notation (JSON) Data Interchange Format".
4. RFC 3986, "Uniform Resource Identifier (URI): Generic Syntax".

### 33.2 Informative References

1. RFC 7322, "RFC Style Guide".
2. W3C Manual of Style and specification conventions.
3. W3C TAG findings on safe web platform design and content security.
4. Unicode Standard Annex #9, "Unicode Bidirectional Algorithm".
5. BCP 47 language tag specifications.

---

## Appendix A. Implementer Checklist

An implementation intended for public use SHOULD publish:

1. supported NODX version and profiles;
2. Canonical AST conformance results;
3. NCP conformance results if claiming `agent-read`;
4. diagnostics JSON conformance results;
5. URL and package safety tests;
6. NODS hostile-style tests if claiming `style`;
7. package corpus tests if claiming `package`;
8. renderer XSS tests if claiming a renderer;
9. resource-limit defaults;
10. CLI exit code mapping if shipping a CLI.

---

## Appendix B. Future Profile Notes

This appendix is informative.

`agent-mutate`, `signature`, `editor`, and `presentation` are reserved because
the ecosystem benefits from stable names before those profiles become mandatory.
A document that requires one of those profiles is not portable across baseline
1.0 processors. Authors who need maximum interoperability SHOULD place those
profiles in `profiles.optional` and provide fallback children or sidecar data.

Future RFCs should define those profiles independently and include their own
conformance fixtures before they are considered stable.

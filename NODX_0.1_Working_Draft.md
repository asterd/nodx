# NODX-WD-0001: NODX 0.1 Working Draft

**Title:** NODX 0.1 — Node-Oriented Document eXchange  
**Document type:** W3C-style Working Draft / standards-track technical proposal  
**Version:** 0.1  
**Date:** 10 May 2026  
**Supersedes:** NODX 1.0 Draft 0.6 Battle-Tested materials  
**Primary extension:** `.nodx`  
**Container model:** hybrid textual-or-packaged, detected by content sniffing  
**Optional package alias:** `.nodz` MAY be used only when a host platform requires a distinct extension for packaged files  
**Style-sheet extension:** `.nods`  
**Component manifest extension:** `.nodc`  
**Proposed media types:** `text/nodx`, `application/nodx+zip`, `text/nodx-style`, `application/nodx-components+json`  
**Status:** Public Draft, not yet a registered standard  

---

## Abstract

NODX, Node-Oriented Document eXchange, is an open, UTF-8, text-first, semantically structured document format for portable source documents. A `.nodx` file can be as simple as a plain text stream and can progressively scale to a bundled ZIP package containing metadata, tables, figures, formulas, reusable components, declarative styles, local assets, change records, signatures, and compact projections for software agents and Large Language Models.

NODX is designed as an authoritative **source and exchange format**, not as a pixel-perfect replacement for final presentation formats. It is intended to interoperate with plain text, Markdown, HTML, PDF, DOCX, ODT, PPTX, and other formats through declared import/export profiles and explicit loss reports. The core format forbids executable code, defaults to offline resource loading, and requires deterministic parsing, validation, rendering fallbacks, and agent-safe mutation primitives.

---

## Status of This Document

This document is a public draft intended to define a coherent NODX 0.1 format that can be implemented independently. It is written in a W3C-style structure but is not a W3C Recommendation and has not yet passed formal wide review, IANA media-type registration, interoperability testing, or security review.

The designation **0.1** is intentional. The earlier materials used “1.0 Draft 0.6”; this version resets the public format identifier to `nodx/0.1` to avoid implying final stability. Future `nodx/1.0` versions SHOULD be assigned only after at least two interoperable parsers, one safe renderer, one package reader, one conformance test suite, and one security corpus have been published.

---

## 1. Conformance and Terminology

### 1.1 Normative Language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **MAY**, and **OPTIONAL** in this document are to be interpreted as described in BCP 14, RFC 2119 and RFC 8174, when, and only when, they appear in all capitals.

### 1.2 Conforming Implementations

A NODX implementation conforms to this draft if it:

1. implements at least one conformance profile defined in Section 4;
2. declares the exact profile identifier it supports, for example `NODX-Plain-0.1`, `NODX-Core-0.1`, or `NODX-Rich+Package-0.1`;
3. preserves unknown attributes and custom component content unless an explicit security policy rejects the document;
4. follows the security requirements in Section 24 for every untrusted input;
5. emits structured diagnostics for parse, validation, policy, and rendering issues.

A conforming processor MAY be a parser, validator, editor, renderer, converter, package reader, signing tool, style engine, LLM projection tool, or agent mutation engine. A processor is not required to implement rendering if it only claims parser conformance.

### 1.3 Document Classes

This specification uses the following terms:

| Term | Meaning |
|---|---|
| **Text NODX** | A `.nodx` file whose bytes are valid UTF-8 source text. |
| **Packaged NODX** | A `.nodx` file whose bytes are a ZIP archive with NODX package structure, manifest, local assets, styles, components, history, and signatures. |
| **Source document** | The NODX text stream before parsing, either read directly from Text NODX or loaded from the package entry of Packaged NODX. |
| **Package** | The ZIP container form of `.nodx`, detected by bytes rather than by a second mandatory extension. |
| **Semantic AST** | The normalized tree used for validation, rendering, signing, conversion, and agent reasoning. |
| **Lossless CST** | A concrete syntax tree preserving exact source bytes, whitespace, line endings, original attribute order, and source ranges for editor round-tripping. |
| **Node** | A typed element in the ordered NODX tree. |
| **Block** | A structural node occupying one or more source lines. |
| **Inline** | A text-level node inside a text-bearing block. |
| **Component** | A declarative extension whose name contains a hyphen and whose fallback is known. |
| **Asset** | A local package entry or permitted data URI referenced by a document. |
| **Renderer** | A processor that maps the AST to HTML, PDF, DOCX, ODT, PPTX, Markdown, TXT, or another output. |
| **Agent** | A human-supervised or autonomous system that reads, summarizes, transforms, or patches a NODX document using stable node identifiers. |

---

## 2. Design Goals and Non-Goals

### 2.1 Goals

NODX 0.1 is designed to:

1. remain readable and editable in ordinary text editors;
2. treat a valid UTF-8 plain text file as a valid Plain Profile document;
3. separate semantic structure from visual layout;
4. support rich document features such as sections, tables, figures, images, formulas, references, footnotes, citations, page breaks, and read-only forms;
5. support print, web, mobile, and responsive rendering through declarative style rules;
6. support internationalization through language tags, writing direction, Unicode text, and bidirectional rendering requirements;
7. support multimodal package assets without requiring executable plugins;
8. support deterministic parsing, canonicalization, signatures, and hashes;
9. support safe custom components with mandatory readable fallbacks;
10. support agent-ready documents through stable node IDs, compact projections, and validated operational changes;
11. be implementable in memory-safe, cross-platform engines without proprietary runtimes;
12. make unsafe behavior explicit and opt-in through host policy.

### 2.2 Non-Goals

NODX 0.1 intentionally does not define:

1. embedded scripts, macros, active content, or Turing-complete plugins;
2. pixel-perfect reproduction of arbitrary PDF, DOCX, or PPTX files;
3. a full spreadsheet calculation engine;
4. a full browser engine or CSS implementation;
5. collaborative network synchronization protocols;
6. a general-purpose database or query language;
7. complex animation timelines, 3D scenes, or interactive application runtimes;
8. a universal legal trust model for signatures.

### 2.3 Market and Applicability Position

NODX is most valuable where a document must be simultaneously human-readable, semantically structured, safe to open, convertible, and suitable for automated review or editing. Strong candidate domains include policy documents, legal templates, technical documentation, AI-agent workflows, knowledge bases, regulated templates, long-lived business documents, and source documents that must export to several final formats.

NODX SHOULD NOT be marketed as a full replacement for every document format in every use case. Its credible role is an open semantic source layer from which existing final formats can be generated, synchronized, or audited.

---

## 3. Versioning, Namespaces, and Media Types

### 3.1 Version Identifier

A NODX 0.1 document uses the schema string:

```yaml
schema: nodx/0.1
```

The schema value has the form:

```abnf
schema-id = "nodx/" major "." minor
major     = 1*DIGIT
minor     = 1*DIGIT
```

A processor claiming `NODX-0.1` MUST accept `nodx/0.1`. It MUST NOT silently treat an unknown future major version as fully compatible. It MAY parse a future minor version in compatibility mode if all required features are known or have safe fallbacks.

### 3.2 File Extensions

NODX 0.1 uses `.nodx` as the primary extension for both textual and packaged documents. Processors MUST determine the physical representation from the first bytes of the file, not from a secondary extension:

1. if the first four bytes are the ZIP local-file-header signature `50 4B 03 04`, the file is Packaged NODX and MUST be processed as Section 18;
2. otherwise, the file is Text NODX and MUST be processed as UTF-8 source text.

The optional `.nodz` extension MAY be used only as a host-platform alias for Packaged NODX when a platform cannot reliably associate a single extension with two media types. `.nodz` is not a separate format. A `.nodz` file MUST contain the same package bytes defined for Packaged NODX.

| Extension | Usage | Proposed media type |
|---|---|---|
| `.nodx` | Text NODX or Packaged NODX, detected by bytes | `text/nodx; charset=utf-8` for text, `application/nodx+zip` for package |
| `.nodz` | Optional alias for Packaged NODX only | `application/nodx+zip` |
| `.nods` | NODX Style Sheet | `text/nodx-style; charset=utf-8` |
| `.nodc` | Component manifest JSON | `application/nodx-components+json` |
| `.ncp.json` | NODX Compact Projection | `application/nodx-ncp+json` |

Until media types are registered, public implementations SHOULD use clearly documented experimental names such as `text/x-nodx` and `application/x-nodx+zip`.

### 3.3 Feature Identifiers

Feature identifiers use lowercase tokens with optional hyphens:

```abnf
feature-name = ALPHA *( ALPHA / DIGIT / "-" )
```

A document MAY declare required and optional features in front matter:

```yaml
requires:
  - rich-tables
  - math
optional:
  - presentation
```

A processor that does not support a required feature MUST reject the document with a structured error unless a declared fallback fully satisfies that feature.

---

## 4. Conformance Profiles

Profiles are additive only where explicitly stated. A tool MAY implement any combination and MUST declare it.

| Profile | Identifier | Scope | Minimum requirements |
|---|---|---|---|
| Plain | `NODX-Plain-0.1` | TXT-first documents | UTF-8, paragraphs, compact headings, safe escaping, implicit metadata. |
| Core | `NODX-Core-0.1` | Structured text documents | Front matter, attributes, delimited blocks, sections, headings, paragraphs, lists, quotes, code, notes, rules, tasks, decisions, core inline syntax, validation diagnostics. |
| Rich | `NODX-Rich-0.1` | Complete semantic documents | Tables, figures, images, embeds, footnotes, citations, bibliography, table of contents, page breaks, formulas, read-only forms, language and direction inheritance. |
| Style | `NODX-Style-0.1` | Declarative presentation | NODS parser, safe style cascade, print/page rules, responsive rules, local font handling. |
| Package | `NODX-Package-0.1` | Multi-file documents | Packaged `.nodx`, manifest, hashes, local assets, safe includes, package limits. |
| Agent | `NODX-Agent-0.1` | LLM and agent workflows | Stable IDs, NCP projection, source maps, operational changes, JSONL change history. |
| Signature | `NODX-Signature-0.1` | Integrity and authenticity | Canonical Semantic AST, package manifest hashing, JWS signatures. |
| Editor | `NODX-Editor-0.1` | Lossless editing | CST preservation, source ranges, trivia preservation, idempotent formatting. |
| Presentation | `NODX-Presentation-0.1` | Slide-like rendering | Slide sections, speaker notes, presentation export hints, PPTX export mapping. |

A Rich implementation MUST also satisfy Core. A Package implementation MUST at least understand Core parsing for its entry document. Style, Agent, Signature, Editor, and Presentation are orthogonal profiles and MAY be combined.

---

## 5. Processing Model

A processor SHOULD implement the following conceptual pipeline:

```text
bytes
  -> UTF-8 reader and resource limits
  -> line normalization for parser, byte preservation for CST
  -> front matter parser
  -> line scanner
  -> block parser
  -> inline parser
  -> raw Semantic AST plus optional Lossless CST
  -> resolver
  -> validator
  -> profile-specific processor, renderer, signer, converter, or agent API
```

### 5.1 Fatal Errors, Errors, Warnings, and Info

Diagnostics have four severities:

| Severity | Meaning |
|---|---|
| `fatal` | Processing cannot continue safely or deterministically. |
| `error` | The document violates the claimed profile; best-effort output MAY be produced only when explicitly requested. |
| `warning` | The document is processable but may lose fidelity, accessibility, or portability. |
| `info` | Non-problematic information useful to editors or integrators. |

A diagnostic object MUST contain at least:

```json
{
  "code": "NODX-E005",
  "severity": "error",
  "message": "Unbalanced block delimiter.",
  "line": 12,
  "column": 1,
  "target": "#node-id"
}
```

`line` and `column` are one-based. When no location exists, the value MUST be `null` or omitted consistently by the implementation.

### 5.2 Determinism

For the same input bytes, policy, and profile set, a conforming parser MUST produce the same AST, diagnostics, and canonical serialization. A renderer MAY produce visually different output on different platforms, but canonical AST hashing MUST be bit-for-bit deterministic.

### 5.3 Loss Handling

Any conversion or projection that omits semantic information SHOULD produce a loss report. Loss reports are REQUIRED for Agent `summary` projections and for import/export to formats that cannot represent all source features.

---

## 6. Source Text Model

### 6.1 Encoding

A `.nodx` file MUST be valid UTF-8. A file containing invalid UTF-8 MUST produce `NODX-E001` with severity `fatal`.

A `.nodx` file MUST NOT contain a Byte Order Mark. A strict processor MUST reject a BOM with `NODX-E018`. A tolerant processor MAY remove a leading BOM only in recovery mode and MUST report a warning.

U+0000 MUST be rejected with `NODX-E002`.

### 6.2 Unicode Normalization

NODX does not require Unicode normalization of text, IDs, attribute values, or asset paths. Processors MUST NOT normalize Unicode text unless a host policy explicitly requests normalization for a specific operation. Identifiers are compared byte-wise after UTF-8 decoding and without case folding.

### 6.3 Line Endings

Input MAY use LF or CRLF. Internal parser events normalize line endings to LF. A canonical serializer MUST emit LF. A Lossless CST MUST preserve the original line-ending sequence for round-trip editing.

### 6.4 Tabs and Spaces

Tabs are allowed in text content. Structural syntax in this draft is defined in terms of column 0 and the ASCII space U+0020. A tab before a block opener prevents that line from being recognized as a block opener.

### 6.5 Resource Limits

Default limits are normative safety defaults. Hosts MAY lower them. Hosts MAY raise them only by explicit policy.

| Parameter | Default limit |
|---|---:|
| Source bytes for a single `.nodx` file | 64 MiB |
| Front matter bytes | 64 KiB |
| Line length | 1 MiB |
| Single attribute value | 64 KiB |
| ID length | 256 UTF-8 bytes |
| Block nesting depth | 32 |
| Inline nesting depth | 32 |
| Nodes per document | 100,000 |
| Data URI size | 5 MiB |
| Expanded AST memory | 64 MiB |
| Include depth | 8 |
| Package uncompressed size | 256 MiB |
| Package file count | 1,024 |

---

## 7. Front Matter

### 7.1 Recognition

Front matter exists only when the first line of the source is exactly three hyphen-minus characters followed by LF or CRLF:

```text
---
```

The front matter ends at the next line that is exactly `---`. Any bytes before the first `---` line mean the document has no front matter and is processed as Plain or Core according to the selected profile.

### 7.2 YAML Safe Subset

Front matter uses a safe subset of YAML 1.2.2. The subset is deliberately close to JSON and common configuration YAML.

Allowed constructs:

1. mappings with string keys;
2. sequences;
3. plain, single-quoted, and double-quoted scalar strings;
4. integers, finite decimals, booleans, and null;
5. ISO 8601 date and date-time strings;
6. simple flow mappings and sequences.

Forbidden constructs:

1. anchors and aliases;
2. explicit tags;
3. merge keys;
4. duplicate keys;
5. multiple YAML documents;
6. binary tags or custom typed objects;
7. non-finite numeric values such as NaN and Infinity;
8. front matter larger than the configured limit.

A Rich processor encountering a forbidden construct MUST produce a `fatal` diagnostic.

### 7.3 Required Fields

A Core document with front matter MUST contain:

| Field | Type | Requirement |
|---|---|---|
| `schema` | string | MUST be `nodx/0.1` for this draft. |
| `type` | string | MUST be `document` unless a profile explicitly defines another root type. |
| `title` | string | REQUIRED for Rich and Package documents; RECOMMENDED for Core. |

### 7.4 Standard Metadata Fields

| Field | Type | Semantics |
|---|---|---|
| `id` | string | Stable document identifier. |
| `title` | string | Human-readable document title. |
| `subtitle` | string | Optional subtitle. |
| `language` | BCP 47 string | Default document language. |
| `dir` | `ltr`, `rtl`, or `auto` | Default base direction. |
| `created` | date-time string | Creation timestamp. |
| `modified` | date-time string | Last modification timestamp. |
| `authors` | sequence | Human or organizational authors. |
| `license` | string or mapping | License identifier or metadata. |
| `keywords` | sequence of strings | Search and discovery terms. |
| `vars` | mapping | Static variables available as `{{vars.name}}`. |
| `styles` | sequence | Local NODS style references. |
| `components` | sequence | Inline component declarations or references. |
| `requires` | sequence of strings | Required features. |
| `optional` | sequence of strings | Optional features. |
| `policies` | mapping | Document-declared policy preferences. Host policy always overrides them. |

Example:

```nodx
---
schema: nodx/0.1
type: document
id: policy-travel-2026
title: Travel Policy
language: en
dir: ltr
vars:
  company: Example Corp
styles:
  - styles/default.nods
components:
  - name: legal-clause
    version: 0.1.0
    fallback: children
requires:
  - rich-tables
  - math
---

# Travel Policy {#title}

Issued by {{vars.company}}.
```

### 7.5 Plain Profile Implicit Metadata

A Plain document without front matter has the following implicit metadata:

```yaml
schema: nodx/0.1
type: document
title: <first heading text or file name or "Untitled">
language: und
dir: auto
```

---

## 8. Nodes and the Semantic AST

### 8.1 Ordered Tree

A NODX document is an ordered tree. Every node has:

| Property | Type | Requirement |
|---|---|---|
| `type` | string | REQUIRED. Core names do not contain hyphens. Custom component names MUST contain at least one hyphen. |
| `id` | string or null | OPTIONAL but REQUIRED for nodes addressed by agents or cross-references. |
| `classes` | ordered set of strings | OPTIONAL. Canonical order is alphabetical. |
| `attrs` | mapping | OPTIONAL. Unknown attributes MUST be preserved. |
| `children` | sequence of nodes | OPTIONAL. |
| `inlines` | sequence of inline nodes | OPTIONAL for text-bearing blocks. |
| `text` | string | OPTIONAL for literal text blocks such as code and math. |
| `sourceRange` | object | OPTIONAL and excluded from canonical signatures. |

### 8.2 Canonical Semantic AST

The Canonical Semantic AST is used for signatures, hashes, deterministic comparison, and agent safety. It excludes:

1. CST trivia;
2. diagnostics;
3. source byte ranges;
4. renderer-computed style results;
5. non-semantic editor state.

It preserves:

1. node ordering;
2. all attributes, including unknown and custom attributes;
3. all text content;
4. all custom components and fallbacks;
5. references and variables as structured nodes, not pre-expanded strings.

### 8.3 Lossless CST

An Editor Profile implementation MUST preserve enough source information to rewrite the original file without unintended changes. A CST SHOULD preserve:

1. original bytes or byte ranges;
2. exact line endings;
3. exact delimiters;
4. original attribute ordering;
5. whitespace between tokens;
6. source comments if a future profile defines them;
7. invalid but recoverable tokens;
8. mapping from CST nodes to AST nodes.

---

## 9. Attribute Syntax

### 9.1 Attribute Blocks

Attributes use a braced block:

```nodx
{#id .class key="value" another="42"}
```

Attribute blocks may appear:

1. at the end of a delimited block opener;
2. at the end of a compact heading line;
3. after a span: `[text]{attrs}`;
4. in locations explicitly defined by future profiles.

A parser MUST accept ID, class, and named attributes in any order. A canonical serializer MUST emit them as:

1. ID;
2. classes in ascending code-point order;
3. named attributes in ascending code-point order by attribute name.

### 9.2 IDs

```abnf
id = ALPHA *( ALPHA / DIGIT / "-" / "_" / "." / ":" )
```

IDs are case-sensitive, byte-wise, and limited to 256 UTF-8 bytes. The ASCII-only identifier rule is deliberate for v0.1 to reduce interop risk. Human-language labels should use `title`, `label`, or text content rather than non-ASCII IDs.

A validator MUST report duplicate IDs as `NODX-E006`. The uniqueness scope is the resolved document after includes.

### 9.3 Classes and Attribute Names

```abnf
class-name = ALPHA *( ALPHA / DIGIT / "-" / "_" )
attr-name  = ALPHA *( ALPHA / DIGIT / "-" / "_" / ":" )
```

Attribute names beginning with `data-` are reserved for application-specific metadata. Attribute names beginning with `aria-` are reserved for accessibility mappings. Unknown attributes MUST be preserved.

### 9.4 Attribute Values

In source syntax, named attribute values are quoted strings:

```nodx
:::note {type="warning" audience="internal"}
Content.
:::
```

A validator MAY coerce values to booleans, numbers, dates, token lists, or enumerations according to the node schema. Coercion MUST NOT destroy the original string value in an Editor Profile implementation.

### 9.5 Common Attributes

| Attribute | Applies to | Values | Meaning |
|---|---|---|---|
| `id` / `#id` | all nodes | ID | Stable identifier. |
| `class` / `.class` | all nodes | class token | Style and semantic grouping. |
| `lang` | all nodes | BCP 47 language tag | Language override. |
| `dir` | all nodes | `ltr`, `rtl`, `auto` | Base direction override. |
| `title` | all nodes | string | Advisory title. |
| `role` | all nodes | token | Semantic role; SHOULD map to platform accessibility roles only when valid. |
| `hidden` | all nodes | `true` or `false` | Hidden from normal rendering. Hidden content remains in the AST. |
| `decorative` | media nodes | `true` or `false` | Marks non-informative media. |
| `alt` | informative media | string | Text alternative. |
| `sensitivity` | all nodes | token | Host-defined access-control hint. |
| `approved` | all nodes | `true` or `false` | Agent mutation control hint. |

---

## 10. Block Syntax

### 10.1 Overview

Blocks are recognized only at column 0 unless a specific compact syntax says otherwise. A line beginning with spaces or tabs is text unless it is inside a literal block whose syntax consumes it.

The block forms in v0.1 are:

1. front matter;
2. delimited blocks;
3. compact headings;
4. compact lists;
5. compact pipe tables in Rich implementations;
6. implicit paragraphs;
7. blank lines.

### 10.2 Delimited Blocks

Canonical form:

```nodx
:::note {#n1 type="warning"}
Warning content here.
:::
```

Rules:

1. The opener starts at column 0.
2. The opener begins with at least three colon characters.
3. The block type **immediately** follows the colons (no separating space). Example: `:::note`. A space between the colons and the name is NOT permitted on an opener.
4. The block type matches `block-type` in the grammar.
5. Optional attributes follow the opener after one ASCII space. Example: `:::note {type="warning"}`.
6. The closer starts at column 0 and consists of the same number of colons used by the opener. The closer takes one of two forms:
   - **Plain close**: colons followed only by ASCII spaces and a line ending. Example: `:::`.
   - **Labelled close**: colons, exactly one ASCII space, then the block type, optionally followed by trailing ASCII spaces and a line ending. Example: `::: note`.
7. A closer MUST close only the top stack frame. It MUST NOT implicitly close intervening frames.
8. A labelled close whose name does not match the open block at the top of the stack MUST emit `NODX-E005` with severity `error`. The block is still closed (recovery), but the diagnostic alerts authors and editors. Plain close form is always accepted regardless of name.
9. Outer blocks SHOULD use more colons than inner blocks when examples or authoring tools generate nested blocks.
10. Labelled closes do not change the AST: the canonical Semantic AST and the canonical JSON of a document with `:::note … ::: note` are byte-identical to the same document written with `:::note … :::`. Labelled closes are intended only as readability sugar for deep nesting.

Example with labelled closes:

```nodx
::::section {#overview}
# Overview

:::note {type="info"}
Nested note.
::: note
:::: section
```

The space between `:::` and the block name is required for the labelled form.
A line such as `::: note` is a labelled close. A line such as `:::note` is an opener of a new block, not a close.

### 10.3 Block Type Names

```abnf
block-type = ALPHA *( ALPHA / DIGIT / "-" )
```

Core block names do not contain hyphens. A block type containing a hyphen is a custom component. An unknown block type without a hyphen is reserved for future NODX versions and SHOULD produce a warning or error according to profile strictness.

### 10.4 Compact Headings

```nodx
# Heading Level 1 {#h1}
## Heading Level 2
```

Compact headings are recognized only at column 0. One to six `#` characters followed by one ASCII space define heading levels 1 through 6. A compact heading maps to a `heading` node with a numeric `level` attribute.

A heading line without a space after the marker is text:

```nodx
#Not a heading
```

### 10.5 Implicit Paragraphs

A maximal sequence of non-blank text lines that is not another recognized block form becomes a `paragraph` node. Blank lines separate paragraphs. A paragraph does not require a final blank line at end of file.

Soft line breaks inside a paragraph are preserved as line-break opportunities. A renderer MAY collapse them to spaces for HTML-like output, unless the paragraph has `preserve-lines="true"`.

### 10.6 Compact Lists

Core processors MUST support compact single-level lists. Nested lists SHOULD use canonical delimited `list` and `item` blocks in v0.1.

Unordered list markers:

```nodx
- First item
- Second item
```

Ordered list markers:

```nodx
1. First item
2. Second item
```

Task list markers:

```nodx
- [ ] Not done
- [x] Done
```

A compact list is a consecutive run of list item lines of compatible kind at column 0. Continuation lines begin with at least two ASCII spaces and are appended to the current item paragraph. A blank line ends the compact list.

Canonical equivalent:

```nodx
:::list {kind="task"}
:::item {checked="false"}
Not done
:::
:::item {checked="true"}
Done
:::
:::
```

### 10.7 Literal Blocks

`code`, `pre`, `math`, and other literal blocks preserve their inner text exactly after line-ending normalization. Inline parsing MUST NOT run inside literal text unless the node definition explicitly permits it.

Example:

```nodx
:::code {lang="rust"}
fn main() {
    println!("hello");
}
:::
```

### 10.8 Void Semantic Blocks

Void semantic blocks such as `toc` and `pagebreak` are represented with an empty delimited block:

```nodx
:::pagebreak
:::
```

A serializer MAY render void blocks on two lines as above. Self-closing block syntax is reserved for future versions and MUST NOT be used in v0.1 canonical output.

---

## 11. Standard Block Nodes

### 11.1 Core Nodes

| Node | Content | Required profile | Semantics |
|---|---|---|---|
| `document` | blocks | Plain | Logical root. |
| `section` | blocks | Core | Structural grouping. |
| `heading` | inlines | Plain/Core | Section or document heading; `level` 1–6. |
| `paragraph` | inlines | Plain/Core | Prose paragraph. |
| `list` | `item` nodes | Core | Ordered, unordered, or task list. |
| `item` | inlines and blocks | Core | List item. |
| `quote` | inlines or blocks | Core | Quotation or quoted block. |
| `code` | literal text | Core | Source code or command text. |
| `pre` | literal text | Core | Preformatted text. |
| `style` | literal text | Style | NODS rules embedded inline; renderer MUST treat content as untrusted CSS and apply the same allowlist as a packaged `.nods` resource. |
| `note` | blocks | Core | Callout, advisory note, warning, tip, etc. |
| `rule` | blocks or empty | Core | Horizontal or semantic separator. |
| `task` | blocks | Core | Action item. |
| `decision` | blocks | Core | Decision record. |

### 11.2 Rich Nodes

| Node | Content | Semantics |
|---|---|---|
| `table` | `row` nodes | Semantic table. |
| `row` | `cell` nodes | Table row. |
| `cell` | inlines or blocks | Table cell; supports `header`, `scope`, `rowspan`, `colspan`. |
| `figure` | media and `caption` | Figure grouping. |
| `caption` | inlines | Figure or table caption. |
| `image` | empty or fallback children | Static image reference. |
| `media` | fallback children | Audio/video or other media metadata; playback is host-policy controlled. |
| `embed` | fallback children | Declarative embedded resource. |
| `include` | empty | Package-local transclusion. |
| `math` | literal text or MathML fragment | Mathematical expression. |
| `toc` | empty | Table of contents placeholder. |
| `footnote` | inlines or blocks | Footnote definition. |
| `bibliography` | `citation-entry` nodes | Bibliography container. |
| `citation-entry` | inlines or structured attrs | Bibliographic item. |
| `pagebreak` | empty | Print page break hint. |
| `form` | read-only fields | Read-only semantic form. |
| `field` | empty or inlines | Read-only field definition. |

### 11.3 Presentation Nodes

The Presentation Profile uses ordinary document structure plus the following nodes:

| Node | Content | Semantics |
|---|---|---|
| `deck` | `slide` nodes | Presentation root or subsection. |
| `slide` | blocks | Slide page. |
| `speaker-notes` | blocks | Non-projected presenter notes. |
| `fragment` | blocks | Incrementally revealed content. Interactive timing is non-normative. |

A Presentation renderer SHOULD map `slide` nodes to PPTX slides, HTML slide sections, or PDF pages. Presentation nodes are not required for normal Rich documents.

---

## 12. Inline Syntax

### 12.1 Inline Forms

| Inline node | Syntax | Semantics |
|---|---|---|
| `strong` | `**text**` | Strong importance. |
| `em` | `*text*` | Emphasis. |
| `code-span` | `` `text` `` | Inline literal code. |
| `link` | `[label](target)` | Hyperlink. |
| `span` | `[text]{attrs}` | Generic inline span. |
| `mark` | `==text==` | Highlighted text. |
| `sub` | `~text~` | Subscript. |
| `sup` | `^text^` | Superscript. |
| `ref` | `@[id]` | Reference to a node ID. |
| `mention` | `@{kind:id}` | Declarative mention. |
| `footnote-ref` | `[^id]` | Footnote reference. |
| `citation-ref` | `[@id]` | Citation reference. |
| `var` | `{{vars.name}}` | Static variable reference. |
| `math-inline` | `$$math$$` | Inline math expression. |

### 12.2 Inline Parsing Rules

1. Inline syntax does not cross block boundaries.
2. Code spans have highest priority and their content is literal.
3. `[^id]` and `[@id]` have priority over links and spans.
4. Variable references are parsed as nodes, not expanded during parsing.
5. `[x](y)` is a link; `[x]{attrs}` is a span; `[x]` without a suffix remains text.
6. `**` has priority over `*`.
7. Unmatched delimiters remain literal text and SHOULD produce no error.
8. Inline nesting depth MUST respect the configured limit.

### 12.3 Escapes

A backslash escapes the following characters in inline content:

```text
\ ` * [ ] ( ) { } # @ ~ ^ = : |
```

A backslash before any other character is preserved as a literal backslash.

### 12.4 Variables

Variables are static string substitutions defined in front matter or package-level variable files. They are never executed and MUST NOT be interpreted as markup after substitution.

Example:

```yaml
vars:
  company: Example Corp
  version: "2.1"
```

```nodx
Issued by {{vars.company}}, version {{vars.version}}.
```

Unresolved variables MUST produce `NODX-E013` with severity `warning` unless the host policy treats unresolved variables as errors.

---

## 13. Tables

### 13.1 Canonical Rich Table

A Rich implementation MUST support canonical table blocks:

```nodx
:::table {#prices caption="Pricing"}
:::row
:::cell {header="true" scope="col"}
Plan
:::
:::cell {header="true" scope="col"}
Price
:::
:::
:::row
:::cell
Basic
:::
:::cell
10 EUR
:::
:::
:::
```

A validator MUST check that `rowspan` and `colspan` values are positive integers and that the resulting table grid is rectangular after spanning is resolved, unless a host policy enables irregular tables.

### 13.2 Compact Pipe Tables

A Rich implementation MAY support compact pipe tables for authoring convenience:

```nodx
| Plan | Price |
| --- | ---: |
| Basic | 10 EUR |
```

If supported, a pipe table MUST round-trip to the same semantic table model as canonical table blocks. A processor that does not support pipe tables MUST treat the lines as paragraphs or produce a profile warning, according to its declared mode.

---

## 14. Figures, Media, and Assets

### 14.1 Images

```nodx
:::image {src="assets/logo.webp" alt="Example Corp logo"}
:::
```

Informative images MUST have a non-empty `alt` attribute. Decorative images MUST set `decorative="true"` and SHOULD use empty `alt=""`.

### 14.2 Figures

```nodx
:::figure {#fig-architecture}
:::image {src="assets/architecture.svg" alt="Architecture diagram"}
:::
:::caption
Reference engine architecture.
:::
:::
```

A `figure` SHOULD contain one or more media nodes and zero or one `caption` node.

### 14.3 Media

The `media` node is declarative. A renderer MAY display a placeholder, metadata, or a playable element only if host policy allows that media type.

```nodx
:::media {src="assets/demo.mp4" type="video/mp4" alt="Demo video"}
This renderer cannot display the video. The demo explains the workflow.
:::
```

The child content is the fallback and MUST be preserved.

### 14.4 Asset References

Asset paths are package-relative POSIX paths. They MUST NOT be absolute, empty, contain `..`, contain backslashes, or normalize outside the package root.

---

## 15. Math

### 15.1 Block Math

```nodx
:::math {notation="tex" display="block" alt="E equals m c squared"}
E = mc^2
:::
```

`notation` values defined by this draft:

| Value | Meaning |
|---|---|
| `tex` | TeX-like math source. |
| `mathml` | Static MathML source. |
| `asciimath` | AsciiMath source. |
| `text` | Plain-text formula. |

A renderer that cannot render the notation MUST render `alt` if present, otherwise it MUST render a safe textual placeholder and preserve the source.

### 15.2 Inline Math

Inline math uses `$$...$$`. Inline math content is literal and does not parse nested inline markup.

```nodx
The mass-energy relation is $$E=mc^2$$.
```

### 15.3 MathML Security

When `notation="mathml"`, processors MUST treat the content as untrusted markup and apply a MathML allowlist. Scriptable, foreign, or active content MUST be rejected or escaped.

---

## 16. Custom Components

### 16.1 Naming

A custom component name MUST contain at least one hyphen:

```nodx
:::legal-clause {#c1 jurisdiction="IT"}
The fallback text remains readable.
:::
```

Names without hyphens are reserved for NODX core and future profiles.

### 16.2 Declaration

Components SHOULD be declared in front matter or in a package component manifest:

```yaml
components:
  - name: legal-clause
    version: 0.1.0
    fallback: children
    schema: components/legal-clause.schema.json
```

### 16.3 Fallbacks

Every component MUST have a safe fallback behavior.

| Fallback | Rendering behavior |
|---|---|
| `children` | Render child nodes normally. |
| `text` | Extract and render plain text. |
| `placeholder` | Render a localized placeholder plus optional title. |
| `none` | Render nothing; allowed only when `decorative="true"`. |

A renderer that does not understand a component MUST preserve its node, attributes, and children and MUST apply the declared fallback.

---

## 17. NODS: NODX Style Sheets

### 17.1 Scope and Design Principles

NODS is a declarative, CSS-like language for rendering hints. NODS is intentionally a strict, closed subset of CSS, not a superset and not a permissive convenience layer.

The reasoning is portability: a NODX document MUST render predictably on browsers, mobile applications, paged-media engines (PDF), terminals (TUI fallback), e-readers, and agents. This is only possible if the styling layer:

1. excludes runtime behaviour (animations, transitions, interactive states);
2. excludes layout escapes (fixed/sticky positioning, viewport-bound geometry, content injection);
3. excludes complex selectors that depend on the full CSS engine;
4. constrains every value to a finite, validated grammar;
5. produces the same observable output across hosts, since signed documents would otherwise carry rendering ambiguity.

A NODS processor that finds a forbidden construct MUST emit `NODX-E027`. In **safe mode** (the default for offline-first, signed, or agent-driven contexts) the offending rule MUST be rejected. In **lenient mode** it MAY be omitted from the computed style and reported as a warning.

Example:

```nods
heading[level="1"] {
  font-size: 24pt;
  font-weight: 700;
  margin-after: 8pt;
}

note[type="warning"] {
  display: callout;
  border-start: 4pt solid #c47f00;
  background-color: #fff8e6;
}

@page {
  size: A4 portrait;
  margin: 18mm;
}
```

### 17.2 Allowed Selectors

NODS v0.1 allows ONLY the following selector forms:

1. node type selector: `paragraph`, `heading`, `note`, ...;
2. ID selector: `#intro`;
3. class selector: `.warning`;
4. attribute-equality selector: `note[type="warning"]`;
5. attribute-presence selector: `note[approved]`;
6. simple descendant combinator: `section note`;
7. selector lists separated by commas: `h1, h2, h3`.

The following CSS selector features are **forbidden** in NODS v0.1 and MUST trigger `NODX-E027`:

| Forbidden | Reason |
|---|---|
| `:hover`, `:focus`, `:active`, `:visited`, `:checked`, `:target` | Interactive states; documents are static. |
| `::before`, `::after`, `::first-line`, `::first-letter`, `::placeholder`, all pseudo-elements | Inject presentational content not present in the AST. |
| `:nth-child(...)`, `:nth-of-type(...)`, `:first-child`, `:last-child` | Positional matching depends on render-time tree inspection. |
| `:not(...)`, `:is(...)`, `:where(...)`, `:has(...)` | Compound matching; expensive and inconsistent across engines. |
| Sibling combinators `+` and `~` | Side-effects of source order beyond direct nesting. |
| Universal selector `*` | Too broad for a styling contract. |
| Attribute substring matchers `[attr*="..."]`, `[attr^="..."]`, `[attr$="..."]` | Pattern matching on user data. |

### 17.3 Property Allowlist

A Style implementation MUST ignore unknown or invalid properties and SHOULD preserve them in Editor Profile round-trips. The v0.1 allowlist includes:

| Category | Properties |
|---|---|
| Display | `display`, `visibility`, `overflow` |
| Typography | `font-family`, `font-size`, `font-weight`, `font-style`, `font-variant`, `line-height`, `letter-spacing`, `word-spacing`, `text-align`, `text-decoration`, `text-transform` |
| Spacing | `margin`, `margin-top`, `margin-right`, `margin-bottom`, `margin-left`, `margin-block-start`, `margin-block-end`, `margin-inline-start`, `margin-inline-end`, `padding`, `padding-top`, `padding-right`, `padding-bottom`, `padding-left`, `gap` |
| Color | `color`, `background-color`, `opacity` |
| Borders | `border`, `border-start`, `border-end`, `border-before`, `border-after`, `border-color`, `border-style`, `border-width`, `border-radius`, `border-inline-start`, `border-inline-end`, `border-block-start`, `border-block-end` |
| Sizing | `width`, `height`, `max-width`, `min-width`, `max-height`, `min-height` |
| Page (paged media) | `size`, `marks`, `bleed`, `page`, `page-break-before`, `page-break-after`, `page-break-inside`, `orphans`, `widows` |
| Tables | `border-collapse`, `cell-padding`, `vertical-align` |
| Presentation | `slide-layout`, `fragment-order` |

The following property families are **forbidden** in NODS v0.1 and MUST trigger `NODX-E027` when present:

| Forbidden property | Reason |
|---|---|
| `position`, `top`, `right`, `bottom`, `left`, `z-index`, `inset` | Layout escapes; break source order and paged media. |
| `transform`, `transform-origin`, `transform-style`, `perspective` | Geometric mutation; defeats deterministic rendering. |
| `transition`, `transition-*`, `animation`, `animation-*`, `will-change` | Time-based behaviour; documents are static. |
| `content` (only when used to inject text) | Generates content not in the AST; violates the AST-as-source-of-truth invariant. |
| `cursor`, `pointer-events`, `user-select` | Interaction semantics. |
| `mask`, `clip`, `clip-path`, `filter`, `backdrop-filter` | Visual-effect surface that complicates print pipelines. |
| `appearance`, `accent-color` | UI-control styling; documents are not forms. |

### 17.4 At-Rules

NODS v0.1 allows ONLY the following at-rules:

| At-rule | Restrictions |
|---|---|
| `@page`, `@page :first`, `@page :left`, `@page :right`, `@page :blank` | Paged-media geometry; see §17.5. |
| `@font-face` | `src` MUST resolve to a package-local font file under `fonts/`. |
| `@import` | MUST reference a package-local `.nods` file. Remote imports are forbidden. |
| `@media print` | Only the `print` media type is recognized in v0.1. |
| `@media (prefers-color-scheme: light \| dark)` | The only media-feature query allowed in v0.1. |

The following at-rules are **forbidden** and MUST trigger `NODX-E027`: `@keyframes`, `@supports`, `@container`, `@layer`, `@property`, `@scope`, `@starting-style`, and any vendor-prefixed at-rule.

### 17.5 Functions and Value Restrictions

Allowed value functions inside NODS:

1. `var(--name)` and `var(--name, fallback)`;
2. `rgb()`, `rgba()`, `hsl()`, `hsla()`;
3. `calc(expr)` where `expr` uses only `+`, `-`, `*`, `/`, parentheses, lengths, `var()`, and bare numbers;
4. `url(...)` ONLY when the URL is a package-local relative path or a fragment.

Forbidden value functions in v0.1: `attr(...)`, `env(...)`, `counter(...)`, `target-counter(...)`, `running(...)`, `image(...)`, `image-set(...)`, `cross-fade(...)`, `paint(...)`, `element(...)`. They MUST trigger `NODX-E027`.

NODS MUST NOT load remote imports. `@import` MAY reference only package-local `.nods` files. `url()` MAY reference only package-local assets unless host policy explicitly allows otherwise. Inline `:::style` blocks MUST be processed by the same allowlist as packaged `.nods` files; processors MUST emit `NODX-E027` and either drop the offending rule (lenient mode) or reject the whole block (safe mode) when forbidden constructs are present, in addition to the renderer-level XSS sanitization (rejection of `</style`, `expression(`, `javascript:` content).

### 17.6 Page Layout

Authors define page geometry through `@page` rules. The reference engine recognizes:

| Property | Values |
|---|---|
| `size` | One of `A3`, `A4`, `A5`, `B5`, `Letter`, `Legal`, `Tabloid` optionally followed by `portrait` or `landscape`, OR two length values `<length> <length>`. |
| `margin` | Single length OR shorthand `top right bottom left`, OR longhand `margin-top`/`margin-right`/`margin-bottom`/`margin-left` (logical synonyms `margin-block-*` and `margin-inline-*` are RECOMMENDED for i18n). |
| `bleed` | A length value applied to all edges; default `0`. |
| `marks` | `none`, `crop`, `cross`, or `crop cross`. |

Named page selectors `@page :first`, `@page :left`, `@page :right`, `@page :blank` MAY be used to override base settings on specific pages. Hosts that do not generate paged output MUST ignore `@page` rules without raising errors.

Example:

```nods
@page { size: A4 portrait; margin: 22mm 18mm; }
@page :first { margin-top: 32mm; }
@page :left { margin-left: 26mm; }
@page :right { margin-right: 26mm; }
```

### 17.7 Box Model

Each block participates in the standard CSS box model. The allowlist exposes:

1. **Padding**: `padding`, per-side longhands, logical-direction synonyms.
2. **Border**: `border` shorthand and `border-color`, `border-style`, `border-width`, `border-radius`. Per-side and per-direction longhands are allowed.
3. **Margin**: `margin`, per-side longhands, logical-direction synonyms.
4. **Sizing**: `width`, `height`, `min-*`, `max-*`.

Authors SHOULD prefer logical-direction properties (`margin-inline-start`, `border-block-end`, etc.) for documents that mix LTR and RTL content.

### 17.8 Spacing and Length Units

Allowed length units:

| Category | Units | Notes |
|---|---|---|
| Absolute | `pt`, `mm`, `cm`, `in`, `pc`, `px` | `1in = 72pt = 25.4mm = 96px`. |
| Relative | `em`, `rem`, `%`, `ex`, `ch` | Resolve against context. |
| Viewport | `vw`, `vh`, `vmin`, `vmax` | Screen-only; ignored on print. |

`gap`, `line-height`, `letter-spacing`, and `word-spacing` accept lengths or unitless multipliers (for `line-height`). Mixing absolute and viewport units in print outputs SHOULD trigger a warning when the document carries an explicit `@page` declaration.

---

## 18. Packaging: Packaged `.nodx`

Packaged NODX is the advanced physical representation of `.nodx`. It is a ZIP archive detected by the ZIP magic bytes and verified by the required `mimetype` entry. The `.nodx` extension is intentionally shared with Text NODX so users can treat NODX as one document format that scales from plain text to bundled documents.

Processors MUST NOT decide whether a `.nodx` file is textual or packaged from the filename alone. A file named `.nodx` MAY be text or ZIP. A file named `.nodz`, if accepted, MUST be ZIP.

### 18.1 ZIP Entry Names

NODX package paths are logical POSIX relative paths without a leading slash. Earlier drafts used `/mimetype`; this draft corrects that to `mimetype` to avoid conflict with safe path rules.

### 18.2 Required Structure

A minimal package contains:

```text
mimetype
manifest.yaml
content/document.nodx
```

Recommended structure:

```text
mimetype
manifest.yaml
content/document.nodx
styles/default.nods
components/components.nodc
assets/
fonts/
vars/global.yaml
history/changes.jsonl
signatures/document.jws
signatures/package.jws
```

### 18.3 `mimetype` Entry

The first ZIP entry MUST be named exactly `mimetype`, MUST be uncompressed, and MUST contain exactly:

```text
application/nodx+zip
```

No trailing newline is allowed in canonical packages.

### 18.4 Manifest

`manifest.yaml` uses the YAML Safe Subset:

```yaml
schema: nodx-package/0.1
entry: content/document.nodx
entries:
  - path: content/document.nodx
    media-type: text/nodx
    size: 12042
    sha256: sha256-BASE64URLDIGEST
  - path: styles/default.nods
    media-type: text/nodx-style
    size: 2048
    sha256: sha256-BASE64URLDIGEST
```

Manifest fields:

| Field | Type | Requirement |
|---|---|---|
| `schema` | string | MUST be `nodx-package/0.1`. |
| `entry` | path | MUST point to the primary `.nodx` entry. |
| `entries` | sequence | MUST list every non-directory entry except optional signature entries according to signing policy. |
| `entries[].path` | path | Package-relative POSIX path. |
| `entries[].media-type` | string | Media type or documented experimental type. |
| `entries[].size` | integer | Uncompressed byte length. |
| `entries[].sha256` | string | Digest of uncompressed bytes, `sha256-` plus base64url without padding. |

### 18.5 Safe ZIP Requirements

A Package processor MUST reject:

1. absolute paths;
2. path components equal to `..`;
3. backslashes in entry names;
4. empty names or names that normalize to empty;
5. duplicate logical paths after normalization;
6. symlinks, hardlinks, device files, and other special files;
7. encrypted entries unless an explicit encryption profile is implemented;
8. unsupported compression methods;
9. ZIP64 unless host policy allows it;
10. decompression ratio above the configured limit;
11. entries whose declared manifest digest does not match actual bytes.

Default package limits:

| Limit | Default |
|---|---:|
| Uncompressed size | 256 MiB |
| File count | 1,024 |
| Path depth | 8 |
| Entry name length | 512 bytes |
| Compression ratio | 100:1 |

---

## 19. Includes

An `include` node transcludes package-local NODX content:

```nodx
:::include {src="content/appendix.nodx"}
:::
```

Rules:

1. `src` MUST be a safe package-relative path.
2. Remote includes are forbidden in v0.1.
3. Include depth MUST NOT exceed the configured limit.
4. Include cycles MUST produce `NODX-E011`.
5. IDs in included documents participate in the resolved document ID namespace unless an explicit prefix is applied by the include processor.
6. A signature profile MUST define whether signatures cover pre-include or post-include ASTs. This draft signs the resolved AST by default.

---

## 20. URLs and Resource Loading

### 20.1 Default Policy

NODX is offline by default. A processor MUST NOT fetch network resources unless host policy explicitly enables network access and restricts permitted schemes and hosts.

### 20.2 URI Handling

Processors MUST parse and resolve URI references according to a standards-compatible URI parser. They MUST NOT rely on string prefix checks alone.

URL policy algorithm:

1. trim leading and trailing ASCII whitespace;
2. reject embedded C0 controls and DEL;
3. percent-decode the scheme area before scheme classification;
4. lowercase the scheme for comparison;
5. reject dangerous or unsupported schemes;
6. normalize package-relative paths;
7. apply host allowlist policy;
8. preserve the original string for Editor Profile round-trips.

### 20.3 Scheme Policy

Default allowed values:

| Use | Default allowed references |
|---|---|
| Links | fragments, relative references, `https`, `http`, `mailto` |
| Assets | package-relative paths, fragments within package assets, permitted static image data URIs |
| Styles | package-relative paths only |
| Includes | package-relative `.nodx` paths only |

Always blocked by default:

```text
javascript:
vbscript:
file:
ftp:
data:text/html
unknown schemes
```

A host MAY block `http` links or remote links entirely.

### 20.4 Data URIs

Data URIs are allowed only for standalone static images in the Rich profile and are capped at 5 MiB by default. Allowed default media types:

```text
image/png
image/jpeg
image/webp
image/svg+xml
```

SVG data URIs are subject to SVG Static Secure Mode.

---

## 21. SVG Static Secure Mode

A renderer that supports SVG MUST sanitize SVG before embedding or rendering it. The sanitizer MUST remove or reject:

1. `script` elements;
2. event-handler attributes;
3. `foreignObject`;
4. external references;
5. remote CSS;
6. remote fonts;
7. animation elements if the host policy disallows dynamic content;
8. unknown elements or attributes outside the selected SVG allowlist.

When safe rendering cannot be guaranteed, the renderer MUST display the `alt` text or fallback content instead of the SVG.

---

## 22. Agent Profile

### 22.1 Stable Addressing

Agent operations MUST address nodes using one of:

1. `#id` target;
2. canonical node path;
3. deterministic node hash;
4. source range only when a Lossless CST source map is available.

Agents SHOULD NOT use fragile plain-text offsets as authoritative edit targets.

### 22.2 NODX Compact Projection

NCP is a JSON projection optimized for LLM context windows, agent reasoning, and deterministic chunking. It is not the authoritative document when the original NODX source is available.

Example:

```json
{
  "schema": "nodx-ncp/0.1",
  "mode": "semantic",
  "sourceHash": "sha256-BASE64URLDIGEST",
  "dict": {
    "t": ["paragraph", "heading", "section", "table", "row", "cell"],
    "a": ["id", "level", "role", "lang"]
  },
  "nodes": [
    [1, "title", {"level": 1}, "Travel Policy"],
    [0, "p1", {}, "Issued by Example Corp."]
  ],
  "chunks": [
    {"id": "chunk-1", "nodes": ["title", "p1"], "sha256": "sha256-BASE64URLDIGEST"}
  ],
  "loss": []
}
```

NCP modes:

| Mode | Loss | Use |
|---|---|---|
| `lossless` | No Semantic AST loss | AST transport and deterministic comparison. |
| `semantic` | Drops CST trivia and computed style | Agent reasoning and review. |
| `summary` | Lossy | Retrieval, previews, and low-token context. |

Every lossy projection MUST include a `loss` array describing omitted information.

### 22.3 Change Records

Agentic changes are stored as JSON Lines in `history/changes.jsonl` inside a package.

Example:

```json
{"schema":"nodx/change/0.1","op":"replace","target":"#payment","author":"agent:legal-reviewer","time":"2026-05-10T10:00:00Z","beforeHash":"sha256-BASE64URLDIGEST","afterHash":"sha256-BASE64URLDIGEST","reason":"Policy update","requiresApproval":true}
```

Required fields:

| Field | Meaning |
|---|---|
| `schema` | MUST be `nodx/change/0.1`. |
| `op` | Operation name. |
| `target` | Stable target address. |
| `author` | Human, service, or agent identifier. |
| `time` | UTC timestamp. |
| `reason` | Human-readable reason. |
| `beforeHash` | REQUIRED for mutating operations. |
| `afterHash` | REQUIRED for mutating operations after application. |

Defined operations:

```text
insert
replace
delete
add-attribute
set-attribute
remove-attribute
add-comment
approve
reject
```

A conforming Agent implementation MUST validate the document after each operation unless host policy explicitly disables validation for batch staging. Failed validation MUST roll back the operation or mark it unapplied.

---

## 23. Signature Profile

### 23.1 Canonicalization

The Signature profile signs the Canonical Semantic AST. Canonicalization steps:

1. parse and validate the document under the claimed profile;
2. resolve package includes according to the signing policy;
3. remove `sourceRange`, CST trivia, diagnostics, and non-semantic editor state;
4. preserve all semantic nodes, text, attributes, unknown attributes, and custom components;
5. sort mapping keys using JSON Canonicalization Scheme rules;
6. serialize to canonical JSON;
7. compute digest and sign.

### 23.2 Hash Format

Hashes use:

```text
sha256-BASE64URL_WITHOUT_PADDING
```

### 23.3 JWS

The recommended signature envelope is JWS. Recommended algorithm: `ES256`. Implementations MAY support `EdDSA`.

JWS protected header:

```json
{
  "alg": "ES256",
  "typ": "nodx-signature+jws",
  "cty": "application/nodx-canonical+json"
}
```

`typ` MUST NOT be `JWT`.

### 23.4 Package Signatures

A package signature SHOULD cover a canonical manifest containing each signed entry path, size, media type, and digest. Assets SHOULD be covered by manifest digests rather than embedded into the JWS payload.

### 23.5 Trust

An embedded public key is not a trust anchor. Trust requires an external host policy, certificate chain, keyring, DID method, transparency log, or other trust framework outside this draft.

---

## 24. Security Requirements

NODX documents are untrusted input. A secure processor MUST:

1. never execute embedded content;
2. never load remote resources by default;
3. enforce UTF-8, line, node, nesting, and memory limits;
4. reject unsafe YAML constructs;
5. reject unsafe ZIP entries;
6. block path traversal;
7. canonicalize and validate URLs;
8. block dangerous schemes;
9. sanitize SVG and MathML;
10. HTML-escape all user text and attributes;
11. avoid browser `innerHTML` with document-derived content;
12. validate package digests when present;
13. preserve unknown attributes without interpreting them as commands;
14. fail closed when policy is ambiguous.

### 24.1 HTML Rendering Security

A browser renderer SHOULD create DOM nodes with safe APIs such as `document.createElement`, `textContent`, and validated `setAttribute`. It MUST NOT feed document-derived strings into `innerHTML`, `outerHTML`, `insertAdjacentHTML`, or equivalent APIs unless the content has been produced by a trusted, audited sanitizer for that exact output context.

Recommended Content Security Policy for generated standalone HTML:

```text
default-src 'none';
img-src 'self' data: blob:;
style-src 'self' 'unsafe-inline';
font-src 'self' data:;
script-src 'none';
base-uri 'none';
form-action 'none';
```

### 24.2 Bidi and Spoofing

Processors SHOULD warn when text contains isolated bidirectional control characters outside an explicit `dir` context. Renderers SHOULD rely on Unicode bidirectional rendering and explicit `dir` attributes rather than reordering stored text.

---

## 25. Accessibility and Internationalization

### 25.1 Language

`language` in front matter and `lang` attributes use BCP 47 language tags. A renderer SHOULD map them to target-format language mechanisms, such as HTML `lang` and PDF language metadata. Specifically, an HTML renderer SHOULD emit `lang` on the root `<html>` element (from the document `language`) and on any block or inline node carrying a `lang` attribute, so that user agents apply correct hyphenation, line-breaking, and font fallback rules.

### 25.2 Direction

`dir` values are `ltr`, `rtl`, and `auto`. Direction inherits from parent nodes. Renderers SHOULD map them to native target-format direction features and SHOULD avoid inserting directional control characters unless required by the target format. An HTML renderer SHOULD emit `dir` on the root `<html>` element (from the document `dir`) and on any node that overrides direction, so the Unicode bidirectional algorithm isolates mixed-script content automatically.

Mixing scripts within a single paragraph is supported: inline spans with `lang` and `dir` attributes (`[٩٨ ريال]{lang="ar" dir="rtl"}`) carry over to the rendered HTML so that browsers and PDF engines apply per-span bidi context.

### 25.3 Accessibility Requirements

NODX processors SHOULD support accessible output by default:

1. informative images require `alt`;
2. decorative images require `decorative="true"`;
3. heading level jumps SHOULD produce warnings;
4. tables SHOULD use `header` and `scope` on header cells;
5. links SHOULD have descriptive labels;
6. read-only fields SHOULD have labels;
7. source order SHOULD match reading order;
8. generated HTML SHOULD be semantic;
9. generated PDF SHOULD be tagged when the renderer supports tagged PDF;
10. presentation exports SHOULD preserve speaker notes separately from slide-visible content.

---

## 26. Interoperability

### 26.1 Recommended Exporters

A mature NODX ecosystem SHOULD provide exporters to:

1. HTML;
2. PDF;
3. DOCX;
4. ODT;
5. Markdown;
6. TXT;
7. PPTX through Presentation Profile;
8. JSON AST and NCP.

### 26.2 Recommended Importers

Importers SHOULD be explicit about confidence and loss:

| Source | Expected fidelity |
|---|---|
| Markdown | High for paragraphs, headings, lists, code, links, and simple tables. |
| HTML | Medium to high for semantic HTML; low for script-heavy pages. |
| DOCX/ODT | Medium to high for structured text; styling may be lossy. |
| PDF | Best-effort reconstruction only; not authoritative. |
| PPTX | Medium for slide layout; low for animations. |

A lossy importer or exporter SHOULD emit a machine-readable report.

---

## 27. Grammar Summary

This ABNF describes the lexical surface. Contextual rules, Unicode text handling, delimiter matching, YAML parsing, and inline nesting are normative in the prose sections above.

```abnf
nodx-document     = [front-matter] *(source-line)

front-matter      = fm-open *(yaml-source-line) fm-close
fm-open           = "---" line-end
fm-close          = "---" line-end

delimited-open    = colons block-type [SP attr-block] line-end
delimited-close   = colons (plain-close / labelled-close) line-end
plain-close       = *SP
labelled-close    = SP block-type *SP
colons            = 3*":"
block-type        = ALPHA *(ALPHA / DIGIT / "-")

compact-heading   = 1*6"#" SP inline-source [SP attr-block] line-end

attr-block        = "{" *SP [attr-token *(1*SP attr-token)] *SP "}"
attr-token        = id-attr / class-attr / named-attr
id-attr           = "#" id
class-attr        = "." class-name
named-attr        = attr-name "=" quoted-string
id                = ALPHA *(ALPHA / DIGIT / "-" / "_" / "." / ":")
class-name        = ALPHA *(ALPHA / DIGIT / "-" / "_")
attr-name         = ALPHA *(ALPHA / DIGIT / "-" / "_" / ":")
quoted-string     = DQUOTE *(attr-char / escape-seq) DQUOTE
attr-char         = %x20-21 / %x23-5B / %x5D-7E / utf8-non-control
escape-seq        = "\\" (DQUOTE / "\\" / "{" / "}" / "[" / "]" / "(" / ")" / ":" / "|" / "n" / "t")

inline-source     = *(inline-char)
source-line       = *(%x09 / %x20-7E / utf8-non-control) line-end

yaml-source-line  = *(%x09 / %x20-7E / utf8-non-control) line-end
line-end          = LF / CRLF
CRLF              = %x0D %x0A
LF                = %x0A
SP                = %x20
ALPHA             = %x41-5A / %x61-7A
DIGIT             = %x30-39
DQUOTE            = %x22
utf8-non-control  = <any valid UTF-8 scalar value except C0 controls, DEL, and U+0000>
inline-char       = <any valid UTF-8 scalar value except U+0000>
```

A parser MUST enforce that a delimited close matches the colon count of the current open block. When the closer uses the labelled form, the parser MUST compare the labelled name with the open block at the top of the stack and emit `NODX-E005` on mismatch. These constraints cannot be expressed directly in ABNF.

---

## 28. Error Registry

| Code | Severity | Description |
|---|---|---|
| `NODX-E001` | fatal | Invalid UTF-8. |
| `NODX-E002` | fatal | U+0000 present. |
| `NODX-E003` | fatal | Unterminated front matter. |
| `NODX-E004` | error | Missing or invalid schema for claimed profile. |
| `NODX-E005` | error | Unbalanced or mismatched block delimiter. |
| `NODX-E006` | error | Duplicate ID. |
| `NODX-E007` | error | Unresolved reference. |
| `NODX-E008` | error | Unresolvable asset. |
| `NODX-E009` | error | Missing required text alternative. |
| `NODX-E010` | error | Path traversal or unsafe package path. |
| `NODX-E011` | error | Include cycle. |
| `NODX-E012` | fatal/error | Resource limit exceeded. |
| `NODX-E013` | warning | Variable referenced but not declared. |
| `NODX-E014` | warning | Custom component not declared. |
| `NODX-E015` | info/warning | Fallback rendering applied. |
| `NODX-E016` | info | Unknown attribute preserved. |
| `NODX-E017` | info/warning | Signature absent or not verified. |
| `NODX-E018` | fatal/warning | Byte Order Mark encountered. |
| `NODX-E019` | fatal | Forbidden YAML construct. |
| `NODX-E020` | error | Unsafe URL or scheme. |
| `NODX-E021` | error | Package digest mismatch. |
| `NODX-E022` | warning | Accessibility issue. |
| `NODX-E023` | warning | Unsupported optional feature. |
| `NODX-E024` | error | Required feature unsupported. |
| `NODX-E025` | error | Table grid invalid. |
| `NODX-E026` | warning | Lossy conversion or projection. |
| `NODX-E027` | error/warning | Forbidden NODS construct (interactive selector, animation, layout escape, or other rule outside the v0.1 allowlist). |

---

## 29. Complete Example

```nodx
---
schema: nodx/0.1
type: document
id: nodx-demo
title: NODX Demo
language: en
dir: ltr
vars:
  company: Example Corp
styles:
  - styles/default.nods
components:
  - name: legal-clause
    version: 0.1.0
    fallback: children
requires:
  - rich-tables
  - math
---

# NODX Demo {#title}

Issued by {{vars.company}}.

:::note {type="warning"}
This is a warning note with **strong** text and a reference to @[tbl-prices].
:::

:::table {#tbl-prices caption="Pricing"}
:::row
:::cell {header="true" scope="col"}
Plan
:::
:::cell {header="true" scope="col"}
Price
:::
:::
:::row
:::cell
Basic
:::
:::cell
10 EUR
:::
:::
:::

The formula $$E=mc^2$$ can be represented inline.

:::math {notation="tex" display="block" alt="Quadratic formula"}
x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}
:::

:::figure {#fig-logo}
:::image {src="assets/logo.webp" alt="Example Corp logo"}
:::
:::caption
Company logo.
:::
:::

:::legal-clause {#clause-1 jurisdiction="EU"}
This fallback text is readable even without the component renderer.
:::
```

---

## 30. References

### 30.1 Normative References

- RFC 2119, “Key words for use in RFCs to Indicate Requirement Levels”.
- RFC 8174, “Ambiguity of Uppercase vs Lowercase in RFC 2119 Key Words”.
- RFC 3986, “Uniform Resource Identifier (URI): Generic Syntax”.
- RFC 5646, “Tags for Identifying Languages”.
- RFC 6838, “Media Type Specifications and Registration Procedures”.
- RFC 7515, “JSON Web Signature (JWS)”.
- RFC 8259, “The JavaScript Object Notation (JSON) Data Interchange Format”.
- RFC 8785, “JSON Canonicalization Scheme (JCS)”.
- YAML 1.2.2 Specification.
- Unicode Standard Annex #9, “Unicode Bidirectional Algorithm”.
- WCAG 2.2, Web Content Accessibility Guidelines.

### 30.2 Informative References

- WHATWG HTML Living Standard.
- CommonMark Specification.
- OASIS OpenDocument Format 1.3.
- ECMA-376 Office Open XML File Formats.
- W3C Process Document and Recommendation Track guidance.

---

## Appendix A. Implementation Readiness Checklist

A public NODX 0.1 implementation SHOULD publish:

1. supported profile declarations;
2. parser and renderer version;
3. default resource limits;
4. security policy defaults;
5. conformance test results;
6. known unsupported optional features;
7. loss behavior for import/export;
8. fuzzing and security corpus status;
9. media-type strategy before registration;
10. signature and trust limitations.

## Appendix B. Rationale for the 0.1 Corrections

This draft intentionally changes several earlier draft details:

1. the public schema is `nodx/0.1`, not `nodx/1.0`, because the format is not final;
2. Core and Rich profiles are separated so simple tools can implement Core without tables, media, math, and forms;
3. package paths no longer start with `/`, because leading slashes conflict with safe path handling;
4. ABNF no longer pretends to enforce stack-sensitive delimiter matching;
5. YAML is specified as a safe subset instead of general YAML;
6. math, i18n, presentation, packaging, agent operations, and signature behavior are explicit enough for independent implementation;
7. security requirements are fail-closed and offline by default.

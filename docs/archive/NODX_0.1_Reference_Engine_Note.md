# NODX-NOTE-0001: NODX 0.1 Reference Engine Technical Note

**Title:** NODX 0.1 Reference Engine — Architecture, Algorithms, APIs, and Hardening Guide  
**Document type:** Non-normative technical note / reference implementation guide  
**Normative reference:** NODX-WD-0001: NODX 0.1 Working Draft  
**Date:** 10 May 2026  
**Status:** Public Draft Technical Note  

---

## Abstract

This document describes a secure, deterministic, cross-platform reference engine for NODX 0.1. It is implementation-oriented and non-normative: when this note conflicts with the NODX 0.1 Working Draft, the Working Draft takes precedence.

The reference engine is designed around a memory-safe core, streaming parsing, strict resource limits, safe package reading, lossless editor round-tripping, deterministic canonicalization, sandboxed rendering, compact LLM projections, and validated agent mutations. Rust is used as the reference language because it supports memory safety, predictable performance, CLI delivery, and WebAssembly bindings, but the architecture is language-neutral.

---

## 1. Engineering Objectives

The reference engine should demonstrate that NODX can be implemented without hidden complexity. It must be small enough for tooling adoption and robust enough for untrusted documents.

Primary objectives:

1. parse Plain and Core documents deterministically;
2. validate Rich, Package, Style, Agent, Signature, Editor, and Presentation profiles as independent modules;
3. preserve source bytes through a Lossless CST for editors;
4. render safe HTML without script execution or DOM XSS;
5. read Packaged `.nodx` ZIP containers without path traversal, special files, duplicate paths, or compression bombs;
6. generate NCP projections for LLM and agent workflows;
7. support stable, validated agent operations and JSONL change records;
8. produce deterministic canonical AST hashes across platforms;
9. expose CLI, Rust, WebAssembly, TypeScript, and Python APIs;
10. publish conformance, fuzzing, and security test results.

Non-objectives of the reference engine v0.1:

1. full browser-grade CSS;
2. pixel-perfect DOCX/PDF/PPTX fidelity;
3. unrestricted media playback;
4. general scripting;
5. cloud collaboration;
6. built-in trust infrastructure for signatures.

---

## 2. Workspace Architecture

Recommended monorepo structure:

```text
nodx/
  crates/
    nodx-core/             # UTF-8 reader, scanner, block parser, inline parser, AST, CST, serializer
    nodx-validate/         # profile validators and accessibility checks
    nodx-style/            # NODS parser, allowlist, cascade, computed style model
    nodx-render-html/      # safe HTML renderer and browser DOM renderer
    nodx-render-pdf/       # sandboxed HTML-to-PDF bridge; native renderer later
    nodx-render-docx/      # DOCX export bridge
    nodx-render-odt/       # ODT export bridge
    nodx-render-pptx/      # Presentation profile exporter
    nodx-render-md/        # Markdown/TXT exporters
    nodx-package/          # safe ZIP reader/writer and manifest verifier
    nodx-agent-sdk/        # operational transforms, policies, change records
    nodx-ncp/              # NODX Compact Projection generator and parser
    nodx-sign/             # canonicalization, hashing, JWS integration
    nodx-wasm/             # WASM bindings
    nodx-cli/              # command-line interface
  packages/
    nodx-js/               # TypeScript package
    nodx-python/           # Python package
  spec/
    grammar.abnf
    tests/
      conformance/
      golden/
      security/
  fuzz/
  corpus/
    xss/
    zip/
    url/
    svg/
    yaml/
```

### 2.1 Crate Boundaries

Crate boundaries reduce the attack surface.

| Crate | Must not depend on | Reason |
|---|---|---|
| `nodx-core` | renderers, ZIP, network, crypto | Parser must be deterministic and side-effect free. |
| `nodx-validate` | renderers, network | Validation must not load resources except through explicit resolver interfaces. |
| `nodx-package` | renderers, network | Package inspection must be safe before extraction. |
| `nodx-render-html` | network | Rendering must not fetch remote resources directly. |
| `nodx-agent-sdk` | LLM APIs | Agent mutation logic must be model-agnostic. |
| `nodx-sign` | renderers | Signatures cover AST/manifest, not output appearance. |

### 2.2 Unsafe Code Policy

The reference implementation should use `#![forbid(unsafe_code)]` in `nodx-core`, `nodx-validate`, `nodx-package`, `nodx-agent-sdk`, and `nodx-ncp`. Any exception must have a written audit note and tests demonstrating why safe Rust was insufficient.

---

## 3. Public API Shape

### 3.1 Rust API

```rust
pub struct ParseOptions {
    pub profile: ProfileSet,
    pub limits: Limits,
    pub preserve_cst: bool,
    pub recovery: RecoveryMode,
}

pub struct ParseResult {
    pub ast: Option<NodxDocument>,
    pub cst: Option<LosslessCst>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn parse_bytes(input: &[u8], options: ParseOptions) -> ParseResult;
pub fn parse_str(input: &str, options: ParseOptions) -> ParseResult;
```

The public parser API must not panic on user input. It should return diagnostics even when the AST is unavailable.

### 3.2 Result Type

```rust
pub enum NodxResult<T> {
    Ok { value: T, diagnostics: Vec<Diagnostic> },
    Err { diagnostics: Vec<Diagnostic> },
}
```

A fatal parse error should use `Err`. Validation errors may use `Ok` with diagnostics when an AST is usable.

### 3.3 Source Ranges

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceRange {
    pub byte_start: u64,
    pub byte_end: u64,
    pub line_start: u32,
    pub column_start: u32,
    pub line_end: u32,
    pub column_end: u32,
}
```

Byte ranges refer to original input bytes. Line and column values are one-based and are intended for diagnostics, not canonical addressing.

---

## 4. Parser Pipeline

```text
input bytes
  -> RepresentationSniffer
  -> PackageReader when ZIP magic is present
  -> Utf8Reader + Limits
  -> LineNormalizer for parser view
  -> FrontMatterScanner
  -> LineScanner
  -> BlockParser
  -> InlineParser
  -> Raw AST + optional CST
  -> Resolver
  -> Validator
  -> Validated AST
```

### 4.1 Utf8Reader

Responsibilities:

1. validate UTF-8;
2. reject U+0000;
3. detect BOM;
4. enforce total byte limit;
5. enforce line length limit;
6. record original byte offsets;
7. produce a normalized LF parser view without losing original bytes.

```rust
pub struct ReadLimits {
    pub max_bytes: usize,
    pub max_line_bytes: usize,
    pub max_front_matter_bytes: usize,
}
```

Implementation notes:

- Validate incrementally rather than allocating a second full copy when possible.
- Track CRLF as one logical line break in parser view and two bytes in source range.
- Reject U+0000 before building AST strings.

### 4.1.1 RepresentationSniffer

The reference engine treats `.nodx` as a hybrid extension. Before UTF-8 validation, it inspects the first bytes:

1. `50 4B 03 04` means Packaged NODX and routes to the safe ZIP PackageReader;
2. every other prefix means Text NODX and routes to UTF-8 validation.

The sniffer must be based on bytes, not the filename. A file named `example.nodx` may be either text or ZIP. Writers should emit both textual and packaged documents as `example.nodx`.

### 4.2 FrontMatterScanner

The scanner checks only the first logical line. If the first line is `---`, it collects until the next exact `---` line or emits `NODX-E003`.

Forbidden YAML features should be detected by a dedicated safe-YAML layer, not by ad hoc regexes.

### 4.3 LineScanner

The scanner emits structural line events but does not parse inline markup.

```rust
pub enum LineEvent<'a> {
    BlockOpen {
        colons: u16,
        node_type: &'a str,
        attrs: Option<&'a str>,
        range: SourceRange,
    },
    BlockClose {
        colons: u16,
        range: SourceRange,
    },
    CompactHeading {
        level: u8,
        content: &'a str,
        attrs: Option<&'a str>,
        range: SourceRange,
    },
    ListItem {
        kind: ListKind,
        ordinal: Option<u32>,
        checked: Option<bool>,
        content: &'a str,
        range: SourceRange,
    },
    PipeTableLine {
        raw: &'a str,
        range: SourceRange,
    },
    TextLine(&'a str, SourceRange),
    BlankLine(SourceRange),
    Eof,
}
```

Scanner invariants:

1. a block opener is recognized only at column 0;
2. a closer is recognized only when the line is colons plus optional spaces;
3. a compact heading requires a space after one to six `#` characters;
4. a tab before a marker makes the line text;
5. malformed structural lines are text unless they create an unsafe ambiguity.

### 4.4 BlockParser

The BlockParser maintains an explicit stack.

```rust
struct Frame {
    colons: u16,
    node_type: String,
    node_path: NodePath,
    open_range: SourceRange,
}
```

Close algorithm:

```rust
if stack.is_empty() {
    emit_error(NODX_E005, close.range);
    treat_as_text_or_recover();
} else if close.colons == stack.last().unwrap().colons {
    close_top_frame();
} else {
    emit_error(NODX_E005, close.range);
    recover_until_matching_close_or_eof();
}
```

The parser must not search the stack for a matching colon count and close multiple frames implicitly. That masks malformed input and breaks deterministic editing.

### 4.5 Paragraph Accumulation

A paragraph buffer starts when the parser receives a `TextLine` outside a literal block and outside a currently active compact list/table parse. It flushes when:

1. a blank line is encountered;
2. a block opener is encountered;
3. a compact heading is encountered;
4. a compact list starts;
5. EOF is reached.

The paragraph buffer stores original line boundaries. The AST can store a normalized string plus line-break metadata.

### 4.6 Compact List Parser

Compact list parsing is intentionally shallow in v0.1.

Rules:

1. compatible consecutive list item events form one `list` node;
2. task items become `item` nodes with `checked` attributes;
3. continuation lines beginning with at least two spaces are appended to the current item;
4. a blank line ends the list;
5. nested compact lists are rejected or treated as continuation text; canonical delimited lists should be used for nesting.

### 4.7 Pipe Table Parser

Rich implementations may enable pipe tables. The parser should detect a header line followed by an alignment separator.

A safe algorithm:

1. candidate line contains at least one unescaped `|`;
2. next line matches separator cells consisting of `-`, `:`, spaces, and pipes;
3. split cells using an escape-aware splitter;
4. trim outer spaces;
5. create `table`, `row`, and `cell` nodes;
6. reject ragged rows unless recovery mode is enabled.

---

## 5. Safe YAML Front Matter

### 5.1 Parser Strategy

Use a YAML parser configured to produce generic events, then validate the event stream against the safe subset. Do not deserialize directly into application structs before checking for forbidden constructs.

Rejected events:

1. alias;
2. anchor;
3. explicit tag;
4. multiple document start/end;
5. duplicate mapping key;
6. merge key;
7. non-string key;
8. non-finite number;
9. binary/custom typed value.

### 5.2 Metadata Struct

```rust
pub struct DocumentMeta {
    pub schema: String,
    pub doc_type: String,
    pub id: Option<String>,
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub language: Option<String>,
    pub dir: Direction,
    pub created: Option<String>,
    pub modified: Option<String>,
    pub authors: Vec<Author>,
    pub vars: IndexMap<String, ScalarValue>,
    pub styles: Vec<String>,
    pub components: Vec<ComponentDecl>,
    pub requires: Vec<String>,
    pub optional: Vec<String>,
    pub policies: IndexMap<String, SafeYamlValue>,
}
```

The parser should preserve unrecognized metadata fields in an `extra` map for round-tripping and signing if they are semantic.

---

## 6. AST and CST Data Structures

### 6.1 Semantic AST

```rust
pub struct NodxDocument {
    pub schema: String,
    pub meta: DocumentMeta,
    pub body: Vec<NodxNode>,
    pub diagnostics: Vec<Diagnostic>,
}

pub struct NodxNode {
    pub node_type: String,
    pub id: Option<String>,
    pub classes: Vec<String>,
    pub attrs: IndexMap<String, AttrValue>,
    pub children: Vec<NodxNode>,
    pub inlines: Vec<InlineNode>,
    pub text: Option<String>,
    pub source_range: Option<SourceRange>,
}

#[serde(untagged)]
pub enum AttrValue {
    Str(String),
    Num(f64),
    Bool(bool),
    List(Vec<String>),
    Null,
}
```

Do not store `unknown_attrs` separately in the canonical AST. Unknown attributes are still semantic because preserving them is required for custom components, signatures, and future compatibility.

### 6.2 Inline AST

```rust
pub enum InlineNode {
    Text(String),
    Strong(Vec<InlineNode>),
    Em(Vec<InlineNode>),
    Code(String),
    Link { label: Vec<InlineNode>, target: String, title: Option<String> },
    Span { children: Vec<InlineNode>, attrs: IndexMap<String, AttrValue> },
    Mark(Vec<InlineNode>),
    Sub(Vec<InlineNode>),
    Sup(Vec<InlineNode>),
    Var { namespace: String, name: String },
    Ref { target: String },
    Mention { kind: String, target: String },
    FootnoteRef { target: String },
    CitationRef { target: String },
    MathInline { source: String, notation: MathNotation },
}
```

### 6.3 Lossless CST

```rust
pub struct LosslessCst {
    pub root: CstNodeId,
    pub nodes: Vec<CstNode>,
    pub trivia: Vec<Trivia>,
    pub source_hash: String,
    pub original_line_endings: LineEndingSummary,
}

pub struct CstNode {
    pub kind: CstKind,
    pub range: SourceRange,
    pub children: Vec<CstNodeId>,
    pub ast_node: Option<NodeId>,
}
```

CST design requirements:

1. lossless source rewrite;
2. source map from AST node to CST ranges;
3. stable incremental edits;
4. preservation of attribute order and spacing;
5. preservation of recoverable malformed syntax for editors.

---

## 7. Attribute Parser

The attribute parser should be a small deterministic scanner, not a regex.

Algorithm:

1. require opening `{` and closing `}`;
2. skip ASCII spaces;
3. parse `#id`, `.class`, or `name="value"`;
4. reject duplicate IDs in one attribute block;
5. allow repeated classes but deduplicate in canonical AST;
6. reject duplicate named attributes unless recovery mode preserves the last value and emits a warning;
7. decode only defined escapes;
8. preserve raw slices in CST.

Error examples:

| Input | Diagnostic |
|---|---|
| `{#}` | invalid ID |
| `{key=value}` | unquoted attribute value |
| `{key="unterminated}` | unterminated string |
| `{#a #b}` | duplicate ID attribute |
| `{x="1" x="2"}` | duplicate named attribute |

---

## 8. Inline Parser

A Pratt parser or a stack-based delimiter parser is acceptable. The implementation must be deterministic and must avoid exponential backtracking.

Priority order:

1. code span;
2. footnote and citation refs;
3. variable refs;
4. mention and node refs;
5. link/span disambiguation;
6. inline math;
7. strong;
8. emphasis;
9. mark/sub/sup;
10. plain text.

### 8.1 Code Spans

Code span content is literal. Escapes, variables, and emphasis are not parsed inside code spans.

### 8.2 Links

A link target should be stored exactly as provided and validated later by the resolver. The inline parser must not fetch or normalize URLs.

### 8.3 Recovery

Unmatched delimiters become literal text. The inline parser should prefer producing a usable paragraph over emitting noisy diagnostics for ordinary prose.

### 8.4 Performance

The inline parser should run in O(n) or O(n log n) time for line length n. Fuzz tests should include repeated delimiter sequences such as `*************` and nested brackets.

---

## 9. Resolver

The resolver transforms a syntactically valid AST into a semantically connected document.

Responsibilities:

1. derive implicit metadata for Plain documents;
2. load package-level variables when allowed;
3. resolve includes with cycle detection;
4. build the ID map;
5. resolve `ref`, `footnote-ref`, and `citation-ref` targets;
6. resolve asset paths through the package reader;
7. apply resource policy to links and assets;
8. annotate unresolved references;
9. prepare the document for validation and rendering.

### 9.1 ResourcePolicy

```rust
pub struct ResourcePolicy {
    pub offline: bool,
    pub allow_remote: bool,
    pub allowed_schemes: HashSet<String>,
    pub allowed_hosts: HashSet<String>,
    pub max_asset_bytes: usize,
    pub max_data_uri_bytes: usize,
    pub allow_http_links: bool,
    pub allow_svg: bool,
    pub allow_media_playback: bool,
}
```

Safe defaults:

```text
offline = true
allow_remote = false
allow_http_links = true
allowed_schemes = {"relative", "fragment", "https", "http", "mailto", "data:image/png", "data:image/jpeg", "data:image/webp", "data:image/svg+xml"}
max_data_uri_bytes = 5 MiB
allow_svg = true with sanitizer
allow_media_playback = false
```

### 9.2 URL Canonicalization

Do not use `starts_with("javascript:")`. Use a parser and a policy function.

Pseudo-code:

```rust
fn classify_uri(raw: &str, policy: &ResourcePolicy) -> UriDecision {
    let trimmed = trim_ascii(raw);
    if contains_control(trimmed) { return Reject(ControlChar); }
    let scheme_candidate = bytes_before_first_colon(trimmed);
    let decoded_scheme = percent_decode_ascii(scheme_candidate);
    let scheme = decoded_scheme.to_ascii_lowercase();
    if is_always_dangerous(&scheme) { return Reject(DangerousScheme); }
    let parsed = parse_uri_reference(trimmed)?;
    apply_policy(parsed, policy)
}
```

Test obfuscations:

```text
javascript:alert(1)
ja%76ascript:alert(1)
java\u0000script:alert(1)
 data:text/html,<script>alert(1)</script>
file:///etc/passwd
..%2f..%2fsecret
```

---

## 10. Validator

Validators should be composable visitors.

```rust
pub trait ValidationRule {
    fn check(&self, doc: &NodxDocument, ctx: &ValidationContext, out: &mut Vec<Diagnostic>);
}
```

### 10.1 Rule Groups

| Group | Examples |
|---|---|
| Syntax-derived | schema present, front matter safe, delimiter matching already checked. |
| Identity | ID uniqueness, valid ID format, stable ID recommendations. |
| References | refs, footnotes, citations, includes. |
| Assets | path safety, existence, digest match, alt text. |
| Tables | row/cell structure, span grid, header scope. |
| Components | hyphenated custom names, declaration, fallback. |
| Variables | declared vars, namespace validity. |
| i18n | valid `lang`, valid `dir`, bidi warnings. |
| Accessibility | alt text, heading jumps, link labels, form labels. |
| Security | URL schemes, SVG policy, MathML policy, package limits. |
| Profile | required feature support, unsupported optional features. |

### 10.2 Strict Mode

Strict mode can promote warnings to errors. Recommended strict promotions:

1. unresolved variable;
2. undeclared component;
3. heading level jump;
4. missing table header scope;
5. remote link in offline-only environment;
6. unsupported optional feature.

---

## 11. Safe Package Reader

The package reader must validate before exposing entries to the resolver.

### 11.1 Open Algorithm

1. accept input only after RepresentationSniffer has identified ZIP magic bytes;
2. read central directory;
3. verify first entry is `mimetype`;
4. verify `mimetype` is stored, not compressed;
5. verify `mimetype` content is exactly `application/nodx+zip`;
6. normalize every entry name as a package POSIX path;
7. reject absolute, empty, `..`, backslash, duplicate, and over-depth paths;
8. reject symlink, hardlink, device, and special-file metadata;
9. reject unsupported compression or encryption;
10. enforce file count, size, and compression ratio limits;
11. read `manifest.yaml` through Safe YAML;
12. verify manifest entries and SHA-256 digests when entries are read or eagerly during open, according to policy;
13. expose a read-only virtual filesystem.

### 11.2 API

```rust
pub trait PackageReader {
    fn open(bytes: &[u8], limits: PackageLimits) -> Result<Self, PackageError>
    where
        Self: Sized;

    fn manifest(&self) -> &PackageManifest;
    fn read_entry(&self, path: &PackagePath) -> Result<Vec<u8>, PackageError>;
    fn exists(&self, path: &PackagePath) -> bool;
    fn list(&self) -> Vec<PackagePath>;
}
```

### 11.3 PackagePath

```rust
pub struct PackagePath(String);

impl PackagePath {
    pub fn parse(input: &str) -> Result<Self, PathError> {
        // reject absolute, empty, backslash, ., .., repeated separators,
        // controls, and excessive length/depth
    }
}
```

Never pass raw ZIP entry names directly to filesystem APIs. Extraction should happen into a virtual filesystem or an isolated temporary directory after validation.

---

## 12. NODS Style Engine

### 12.1 Parser

Use a dedicated small CSS-like parser. Reusing a full CSS engine is acceptable only if selectors, properties, URLs, and imports are constrained by an allowlist before evaluation.

### 12.2 Cascade Model

Recommended cascade order:

1. UA/default NODX styles;
2. package styles in declared order;
3. document styles in declared order;
4. inline style attributes if a future profile allows them;
5. host accessibility overrides.

Host accessibility overrides must win over author styling.

### 12.3 Property Parsing

Every property has a dedicated value parser. Invalid values are ignored and produce diagnostics.

Examples:

```rust
enum Display { Block, Inline, None, List, Table, TableRow, TableCell, Callout, Figure, Slide }
enum Length { Pt(f32), Px(f32), Mm(f32), Em(f32), Rem(f32), Percent(f32) }
```

### 12.4 Import Handling

`@import` is package-local only. The style engine must enforce import depth and cycle detection.

---

## 13. HTML Renderer

### 13.1 Output Strategy

The safe renderer should write HTML using an escaping writer or construct DOM nodes directly. It must maintain separate contexts for:

1. text nodes;
2. attributes;
3. URLs;
4. CSS values;
5. raw sanitized SVG or MathML fragments.

### 13.2 Escaping

```rust
fn escape_html_text(input: &str) -> String {
    input.chars().flat_map(|c| match c {
        '&' => "&amp;".chars().collect::<Vec<_>>(),
        '<' => "&lt;".chars().collect::<Vec<_>>(),
        '>' => "&gt;".chars().collect::<Vec<_>>(),
        _ => vec![c],
    }).collect()
}

fn escape_html_attr(input: &str) -> String {
    input.chars().flat_map(|c| match c {
        '&' => "&amp;".chars().collect::<Vec<_>>(),
        '<' => "&lt;".chars().collect::<Vec<_>>(),
        '>' => "&gt;".chars().collect::<Vec<_>>(),
        '"' => "&quot;".chars().collect::<Vec<_>>(),
        '\'' => "&#x27;".chars().collect::<Vec<_>>(),
        _ => vec![c],
    }).collect()
}
```

### 13.3 DOM Renderer

In browser/WASM mode:

```typescript
const el = document.createElement("p");
el.textContent = nodeText;
```

Do not assign document-derived strings to `innerHTML`, `outerHTML`, or `insertAdjacentHTML`.

### 13.4 Node Mapping

| NODX node | HTML target |
|---|---|
| `section` | `section` |
| `heading[level=1]` | `h1` |
| `paragraph` | `p` |
| `list` | `ul`, `ol`, or task-list structure |
| `quote` | `blockquote` |
| `code` | `pre > code` |
| `note` | `aside` with role mapping when valid |
| `table` | `table` |
| `row` | `tr` |
| `cell[header=true]` | `th` |
| `cell` | `td` |
| `image` | `img` |
| `figure` | `figure` |
| `caption` | `figcaption` or `caption` by context |
| `math` | sanitized MathML or text fallback |
| `pagebreak` | print CSS break marker |

---

## 14. PDF Renderer

### 14.1 v0.1 Recommended Pipeline

```text
Validated AST
  -> safe HTML + safe CSS
  -> sandboxed headless renderer with network disabled
  -> PDF
```

Sandbox requirements:

1. network disabled;
2. JavaScript disabled where the engine allows it;
3. isolated temporary profile;
4. no filesystem access except validated package assets and temp output;
5. timeout;
6. memory limit;
7. CSP applied;
8. local package fonts only after validation.

### 14.2 Native Renderer Roadmap

A later native renderer should target:

1. deterministic output;
2. tagged PDF;
3. PDF/A profile;
4. PDF/UA accessibility;
5. reproducible signatures;
6. no browser dependency.

---

## 15. Other Exporters

### 15.1 Markdown/TXT

Markdown export should be lossy-aware. Unsupported components should render fallback children with comments in the loss report, not raw unknown syntax.

TXT export should produce a readable linearization with headings, list markers, table approximations, and alt text for images.

### 15.2 DOCX/ODT

DOCX and ODT export should map semantic nodes to native paragraph styles, table structures, relationships, alt text, language, and section breaks. Exporters should not attempt to preserve arbitrary NODS rules as native styles unless the mapping is explicit.

### 15.3 PPTX

The Presentation exporter should map `slide` to slides, `speaker-notes` to notes, headings to title placeholders when possible, and figures/tables to native drawing/table objects. Unsupported slide animations should be reported as loss.

---

## 16. Math Renderer

Recommended stages:

1. parse or classify notation;
2. enforce length and complexity limits;
3. render TeX/AsciiMath through a sandboxed or pure library;
4. sanitize generated MathML or SVG;
5. preserve source and alt text;
6. report unsupported commands or lossy conversion.

Math rendering must not execute TeX commands that read files, write files, spawn processes, load network resources, or define arbitrary macros beyond a safe limit.

---

## 17. SVG and Media Handling

### 17.1 SVG Sanitizer

The sanitizer should use a structural XML parser and an allowlist. Regex-based sanitization is not sufficient.

Minimum rejection set:

1. script elements;
2. event attributes;
3. foreignObject;
4. external hrefs;
5. remote CSS;
6. remote fonts;
7. data URLs except allowed static images under policy;
8. animation if disallowed;
9. entity expansion attacks.

### 17.2 Media

Media playback is off by default. A renderer can show metadata, poster image, and fallback text without enabling playback.

---

## 18. NODX Compact Projection

### 18.1 API

```rust
pub enum NcpMode {
    Lossless,
    Semantic,
    Summary,
}

pub struct NcpOptions {
    pub mode: NcpMode,
    pub max_chunk_chars: usize,
    pub include_hashes: bool,
    pub include_styles: bool,
    pub dictionary: bool,
    pub include_source_map: bool,
}

pub fn project(doc: &NodxDocument, options: NcpOptions) -> Result<NcpDocument, NcpError>;
```

### 18.2 Projection Algorithm

1. verify the AST is valid enough for the chosen mode;
2. remove fields excluded by mode;
3. canonicalize node IDs and paths;
4. build dictionaries for node types, attribute names, and repeated strings;
5. serialize nodes as compact tuples;
6. chunk by section boundaries where possible;
7. enforce `max_chunk_chars` without splitting inside atomic nodes unless necessary;
8. compute chunk hashes if requested;
9. produce source map from node IDs to chunk IDs and offsets;
10. produce a `loss` array for omitted information.

### 18.3 Loss Report

```json
{
  "kind": "style-omitted",
  "target": "#intro",
  "reason": "mode semantic excludes computed styles"
}
```

Loss reports should be deterministic and stable so agents can reason about missing context.

---

## 19. Agent Mutation SDK

### 19.1 Policy

```rust
pub struct AgentPolicy {
    pub can_modify_approved: bool,
    pub allowed_sensitivity: HashSet<String>,
    pub validate_after_op: bool,
    pub require_ids_for_inserted_blocks: bool,
    pub allow_source_range_targets: bool,
    pub require_human_approval_for: HashSet<String>,
}
```

Safe defaults:

```text
can_modify_approved = false
validate_after_op = true
require_ids_for_inserted_blocks = true
allow_source_range_targets = false
```

### 19.2 Operations

```rust
replace_block(doc, target, new_node, meta, policy)
insert_block(doc, target, position, new_node, meta, policy)
delete_block(doc, target, meta, policy)
set_attribute(doc, target, attr, value, meta, policy)
remove_attribute(doc, target, attr, meta, policy)
approve_change(doc, change_id, meta, policy)
reject_change(doc, change_id, meta, policy)
```

### 19.3 Operation Algorithm

Each mutating operation:

1. resolves target;
2. checks policy;
3. computes `beforeHash` for the smallest stable affected subtree;
4. applies the operation to a clone or transactional edit buffer;
5. validates the result;
6. rolls back on fatal/error unless staging mode is enabled;
7. computes `afterHash`;
8. emits a ChangeRecord;
9. updates source map if CST editing is enabled.

### 19.4 Hallucination Resistance

Agents often invent offsets, IDs, or nearby text. The SDK should fail closed when:

1. target ID does not exist;
2. target hash does not match;
3. source range is stale;
4. multiple nodes match an ambiguous target;
5. validation fails after edit;
6. the agent tries to modify approved or sensitive content without permission.

---

## 20. Canonicalization and Signatures

### 20.1 Canonical AST Function

```rust
pub fn canonical_ast_for_signing(doc: &NodxDocument) -> CanonicalValue {
    // remove sourceRange, CST trivia, diagnostics, editor-only state
    // preserve all semantic attrs, including unknown/custom attrs
    // preserve custom components and fallback content
    // sort object keys according to JCS-compatible canonical JSON
}
```

### 20.2 Hashing

```rust
pub fn semantic_hash(doc: &NodxDocument) -> String {
    let canonical = canonical_json(canonical_ast_for_signing(doc));
    format!("sha256-{}", base64url_no_pad(sha256(canonical.as_bytes())))
}
```

### 20.3 JWS Integration

The signature crate should expose signing and verification without controlling trust policy.

```rust
pub fn sign_document(doc: &NodxDocument, key: &SigningKey) -> Result<Jws, SignError>;
pub fn verify_document(doc: &NodxDocument, jws: &Jws, key: &VerificationKey) -> VerificationResult;
```

Verification result should distinguish:

1. cryptographic validity;
2. digest mismatch;
3. unsupported algorithm;
4. missing key;
5. untrusted key;
6. expired or revoked certificate if host policy checks it.

---

## 21. WASM, TypeScript, and Python Bindings

### 21.1 WASM Rules

WASM APIs must not panic. They should return structured result objects.

```rust
#[wasm_bindgen]
pub fn parse(input: &str, options: JsValue) -> JsValue {
    match parse_options(options) {
        Ok(opts) => to_js(parse_str(input, opts)),
        Err(diag) => to_js_error(vec![diag]),
    }
}
```

### 21.2 TypeScript API

```typescript
export type NodxResult<T> =
  | { ok: true; value: T; diagnostics: Diagnostic[] }
  | { ok: false; diagnostics: Diagnostic[] };

export interface ParseOptions {
  profile?: string[];
  preserveCst?: boolean;
  recovery?: "none" | "safe";
  limits?: Partial<Limits>;
}
```

### 21.3 Python API

```python
from nodx import parse, validate, render_html

result = parse(text, profile=["core"], preserve_cst=True)
if result.ok:
    html = render_html(result.value)
else:
    print(result.diagnostics)
```

Python bindings should not expose unsafe package extraction helpers.

---

## 22. CLI

Recommended commands:

```bash
nodx validate file.nodx
nodx format file.nodx --write
nodx render file.nodx --to html --out file.html
nodx render file.nodx --to pdf --out file.pdf
nodx render file.nodx --to docx --out file.docx
nodx render file.nodx --to pptx --out deck.pptx
nodx package file.nodx --out bundled.nodx
nodx inspect bundled.nodx
nodx ncp file.nodx --mode semantic --out context.ncp.json
nodx diff old.nodx new.nodx --json
nodx patch file.nodx changes.jsonl --out patched.nodx
nodx scrub bundled.nodx --remove-history --out public.nodx
nodx sign bundled.nodx --key private.pem
nodx verify bundled.nodx --key public.pem
```

Exit codes:

| Code | Meaning |
|---:|---|
| 0 | Success. |
| 1 | Validation errors. |
| 2 | Fatal parse or package error. |
| 3 | System error. |
| 4 | Configuration or policy error. |
| 5 | Signature verification failure. |
| 6 | Unsupported profile or feature. |

CLI output should support `--json` for machine-readable diagnostics.

---

## 23. Test Suite

### 23.1 Categories

1. UTF-8 and line endings;
2. front matter safe YAML;
3. attributes;
4. block parsing;
5. compact headings;
6. compact lists;
7. paragraphs;
8. inline parsing;
9. tables;
10. math;
11. images and assets;
12. components and fallbacks;
13. package security;
14. URL policy;
15. SVG sanitizer;
16. HTML escaping;
17. NODS parsing and cascade;
18. NCP projection;
19. agent mutations;
20. signatures;
21. CST round-trip;
22. import/export loss reports;
23. fuzz regressions.

### 23.2 Test Case Format

```json
{
  "id": "parser-block-note-001",
  "profile": ["core"],
  "input": ":::note {type=\"warning\"}\nWarning\n:::\n",
  "expectedDiagnostics": [],
  "expectedAstContains": {
    "type": "note",
    "attrs": {"type": "warning"},
    "text": "Warning"
  }
}
```

### 23.3 Golden Tests

Golden tests should cover:

1. AST JSON;
2. canonical JSON;
3. semantic hash;
4. HTML output;
5. NCP output;
6. formatted source output;
7. CST round-trip output.

Golden outputs must be deterministic across supported platforms.

---

## 24. Fuzzing and Security Corpus

Minimum release criteria before calling an implementation production-ready:

| Target | Minimum |
|---|---:|
| `nodx-core` parser fuzz | 72 hours without panic, memory error, or timeout class bug. |
| `nodx-package` fuzz | 72 hours without unsafe extraction or panic. |
| `nodx-style` fuzz | 24 hours without panic. |
| Attribute parser fuzz | 24 hours without panic. |
| Inline parser fuzz | 24 hours without exponential behavior. |
| HTML XSS corpus | 500 cases escaped or rejected. |
| URL obfuscation corpus | 300 cases correctly classified. |
| ZIP bomb/path traversal corpus | 100 cases rejected. |
| YAML unsafe corpus | 100 cases rejected. |
| SVG sanitizer corpus | 100 malicious SVGs rejected or sanitized. |
| Core line coverage | >= 90%. |
| Validator line coverage | >= 95%. |

Security corpus failures must block release.

---

## 25. Benchmarks

Suggested targets on a modern laptop-class CPU:

| Scenario | Target |
|---|---:|
| Parse 10,000 simple blocks | <= 100 ms |
| Validate 10,000 simple blocks | <= 150 ms |
| Render HTML for 10,000 simple blocks | <= 250 ms |
| Generate NCP semantic projection for 10,000 simple blocks | <= 120 ms |
| Open package with 100 entries and verified hashes | <= 150 ms |
| WASM parser gzip size | <= 2 MiB |
| Peak memory for 10 MiB source | <= 5x source size in AST+CST mode |

Benchmarks should publish hardware, compiler version, optimization flags, and corpus details.

---

## 26. Hardening Checklist

Before release, verify:

1. no public API panics on malformed input;
2. no renderer uses unsafe HTML insertion with document content;
3. package reader rejects absolute paths, traversal, duplicates, symlinks, devices, and bombs;
4. URL policy catches percent-encoded and control-character obfuscation;
5. Safe YAML rejects aliases, anchors, tags, merge keys, and duplicate keys;
6. SVG sanitizer is structural and allowlist-based;
7. Math renderer cannot read files, write files, execute commands, or load network resources;
8. resource limits are applied before large allocation;
9. canonical hashes are stable on x86_64, ARM64, and WASM;
10. unknown attributes are preserved and signed;
11. agent mutations roll back on validation failure;
12. all lossy conversions produce loss reports;
13. fuzz corpora are part of CI;
14. dependency audit is run for every release;
15. unsafe code audit, if any, is published.

---

## 27. Implementation Pitfalls

### 27.1 Treating NODX as Markdown

NODX borrows authoring convenience from lightweight markup, but it is not Markdown. Do not use a Markdown parser and patch the result. NODX requires stable IDs, block delimiters, a semantic AST, source maps, and security policies that Markdown parsers do not provide by default.

### 27.2 General YAML Deserialization

Do not deserialize front matter directly into rich language objects. Always validate the safe YAML subset first.

### 27.3 ZIP Extraction Before Validation

Do not extract first and validate later. Validate central directory and entry paths before any filesystem write.

### 27.4 Stack Search on Block Close

Do not close the nearest matching delimiter deeper in the stack. Only the top frame can close. Anything else is malformed.

### 27.5 URL Prefix Checks

Do not use raw string prefix checks for dangerous schemes. Decode and parse first.

### 27.6 Unknown Attributes

Do not drop unknown attributes. They may be required by future profiles, custom components, accessibility, or signatures.

---

## 28. Roadmap

| Phase | Deliverable |
|---:|---|
| 0 | NODX 0.1 Working Draft, Technical Note, grammar, test schema. |
| 1 | `nodx-core` Plain/Core parser and serializer. |
| 2 | Validator and conformance runner. |
| 3 | Safe HTML renderer. |
| 4 | Package reader/writer and manifest verifier. |
| 5 | NCP projection library. |
| 6 | WASM and TypeScript bindings. |
| 7 | CLI validate/format/render/ncp. |
| 8 | NODS style engine. |
| 9 | Sandboxed PDF renderer. |
| 10 | Agent SDK and JSONL change history. |
| 11 | DOCX/ODT/Markdown/TXT converters. |
| 12 | Signature profile implementation. |
| 13 | Presentation/PPTX exporter. |
| 14 | VS Code extension or Language Server Protocol integration. |

---

## 29. Public Draft Definition of Done

A public reference release should not claim production readiness until:

1. parser and validator pass all Core conformance tests;
2. HTML renderer passes the XSS corpus;
3. package reader blocks traversal, duplicates, symlinks, devices, and bombs;
4. NCP output is deterministic bit-for-bit;
5. formatter and parser are idempotent on the golden corpus;
6. WASM API does not panic on fuzzed input;
7. at least one CLI and one HTML renderer are installable;
8. media-type registration strategy is documented;
9. examples cover Plain, Core, Rich, Package, Agent, and Signature profiles;
10. security limitations are clearly disclosed.

---

## 30. Example End-to-End Flow

```text
User opens demo.nodx
  -> Reader sniffs first bytes
  -> Text NODX goes directly to UTF-8 parser
  -> Packaged NODX goes to PackageReader
  -> PackageReader validates ZIP structure and manifest
  -> entry content/document.nodx loaded from VFS
  -> Parser creates AST+CST
  -> Resolver resolves includes, variables, assets, IDs
  -> Validator emits warnings for accessibility or unresolved optional features
  -> HTML renderer creates safe DOM or escaped HTML
  -> Agent SDK receives NCP semantic projection
  -> Agent proposes replace operation on #clause-1
  -> SDK checks beforeHash, applies clone edit, validates, computes afterHash
  -> ChangeRecord appended to history/changes.jsonl
  -> Signature profile invalidates old signature until re-signed
```

---

## 31. Conclusion

The reference engine should prove the central NODX claim: a modern document source format can be text-first, semantic, portable, secure by default, and agent-ready without becoming a browser, office suite, or scripting runtime. The success criteria are not only syntax elegance but also deterministic behavior, safe failure modes, testability, and clear interoperability boundaries.

use std::collections::BTreeMap;

use crate::ResourceLimits;
use crate::ast::{Attrs, Document, Node, Value};
use crate::attrs::{
    parse_attrs, parse_heading, parse_heading_with_cap, parse_matching_close, parse_opener,
    parse_opener_with_cap, valid_name,
};
use crate::diagnostic::{Diagnostic, diag};
use crate::front_matter::parse_front_matter;
use crate::inline_parser::parse_inlines;

pub fn parse_str(input: &str) -> Document {
    parse_str_with_limits(input, ResourceLimits::default())
}

pub fn parse_str_with_limits(input: &str, limits: ResourceLimits) -> Document {
    let mut diagnostics = Vec::new();
    if input.starts_with('\u{feff}') {
        diagnostics.push(diag(
            "NODX-E018",
            "fatal",
            "Byte Order Mark is not allowed.",
            1,
            1,
        ));
    }
    if input.contains('\0') {
        diagnostics.push(diag("NODX-E002", "fatal", "U+0000 is not allowed.", 1, 1));
    }
    if input.len() > limits.source_bytes {
        diagnostics.push(diag(
            "NODX-E012",
            "fatal",
            "Input byte size limit exceeded.",
            1,
            1,
        ));
    }
    if let Some((line, _len)) = first_oversize_line(input, limits.line_length) {
        diagnostics.push(diag(
            "NODX-E012",
            "fatal",
            "Line length limit exceeded.",
            line,
            1,
        ));
    }
    if input.starts_with("---")
        && (input.len() == 3 || matches!(input.as_bytes().get(3), Some(b'\n' | b'\r')))
        && let Some(bytes) = front_matter_region_bytes(input)
        && bytes > limits.front_matter_bytes
    {
        diagnostics.push(diag(
            "NODX-E012",
            "fatal",
            "Front matter size limit exceeded.",
            1,
            1,
        ));
    }
    if diagnostics.iter().any(|d| d.severity == "fatal") {
        return Document {
            schema: "nodx/1.0".to_string(),
            meta: default_meta(false),
            body: Vec::new(),
            diagnostics,
        };
    }

    let normalized = input.replace("\r\n", "\n");
    let mut lines: Vec<&str> = normalized.split('\n').collect();
    if lines.last() == Some(&"") {
        lines.pop();
    }

    let mut meta = BTreeMap::new();
    let mut start = 0;
    let mut had_front_matter = false;
    if lines.first() == Some(&"---") {
        match lines.iter().skip(1).position(|line| *line == "---") {
            Some(end_rel) => {
                had_front_matter = true;
                let end = end_rel + 1;
                let front_matter_bytes = lines[1..end].iter().map(|line| line.len()).sum::<usize>();
                if front_matter_bytes > limits.front_matter_bytes {
                    diagnostics.push(diag(
                        "NODX-E012",
                        "fatal",
                        "Front matter size limit exceeded.",
                        1,
                        1,
                    ));
                }
                meta = parse_front_matter(&lines[1..end], &mut diagnostics);
                start = end + 1;
            }
            None => diagnostics.push(diag("NODX-E003", "fatal", "Unclosed front matter.", 1, 1)),
        }
    }
    for (key, value) in default_meta(had_front_matter) {
        meta.entry(key).or_insert(value);
    }
    if !had_front_matter {
        let mut profiles = BTreeMap::new();
        profiles.insert(
            "requires".to_string(),
            Value::List(vec![Value::String("core".to_string())]),
        );
        meta.entry("profiles".to_string())
            .or_insert(Value::Map(profiles));
    }

    let mut parser = Parser {
        lines: &lines,
        pos: start,
        diagnostics,
        limits,
        node_count: 0,
        line_offset: 0,
        depth: 0,
        nodes_limit_hit: false,
        depth_limit_hit: false,
    };
    let body = parser.parse_until(None);
    let diagnostics = parser.diagnostics;

    Document {
        schema: "nodx/1.0".to_string(),
        meta,
        body,
        diagnostics,
    }
}

fn default_meta(had_front_matter: bool) -> BTreeMap<String, Value> {
    let mut meta = BTreeMap::from([
        ("schema".to_string(), Value::String("nodx/1.0".to_string())),
        ("type".to_string(), Value::String("document".to_string())),
        ("dir".to_string(), Value::String("auto".to_string())),
        ("language".to_string(), Value::String("und".to_string())),
    ]);
    if !had_front_matter {
        let mut profiles = BTreeMap::new();
        profiles.insert(
            "requires".to_string(),
            Value::List(vec![Value::String("core".to_string())]),
        );
        meta.insert("profiles".to_string(), Value::Map(profiles));
    }
    meta
}

fn first_oversize_line(input: &str, limit: usize) -> Option<(usize, usize)> {
    let mut line = 1usize;
    let mut len = 0usize;
    for byte in input.bytes() {
        match byte {
            b'\n' => {
                line += 1;
                len = 0;
            }
            b'\r' => {}
            _ => {
                len += 1;
                if len > limit {
                    return Some((line, len));
                }
            }
        }
    }
    None
}

fn front_matter_region_bytes(input: &str) -> Option<usize> {
    let normalized = input.replace("\r\n", "\n");
    let mut bytes = 0usize;
    for (idx, line) in normalized.split('\n').enumerate() {
        if idx == 0 {
            if line != "---" {
                return None;
            }
            continue;
        }
        if line == "---" {
            return Some(bytes);
        }
        bytes += line.len();
    }
    None
}

struct Parser<'a> {
    lines: &'a [&'a str],
    pos: usize,
    diagnostics: Vec<Diagnostic>,
    limits: ResourceLimits,
    node_count: usize,
    line_offset: usize,
    depth: usize,
    nodes_limit_hit: bool,
    depth_limit_hit: bool,
}

impl Parser<'_> {
    fn line_no(&self, pos: usize) -> usize {
        self.line_offset + pos + 1
    }

    fn count_node(&mut self, line: usize) -> bool {
        self.node_count += 1;
        if self.node_count > self.limits.nodes_per_document {
            if !self.nodes_limit_hit {
                self.diagnostics.push(diag(
                    "NODX-E012",
                    "fatal",
                    "Node count limit exceeded.",
                    line,
                    1,
                ));
                self.nodes_limit_hit = true;
            }
            return false;
        }
        true
    }

    fn parse_until(&mut self, close_frame: Option<(usize, &str)>) -> Vec<Node> {
        let mut out = Vec::new();
        let mut closed = close_frame.is_none();
        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            if line.len() > self.limits.line_length {
                self.diagnostics.push(diag(
                    "NODX-E012",
                    "error",
                    "Line length limit exceeded.",
                    self.line_no(self.pos),
                    1,
                ));
            }
            if let Some((n, expected_name)) = close_frame {
                if let Some(label) = parse_matching_close(line, n, expected_name) {
                    if let Some(name) = label
                        && name != expected_name
                    {
                        self.diagnostics.push(diag(
                            "NODX-E005",
                            "error",
                            &format!(
                                "Closing label `{}` does not match open block `{}`.",
                                name, expected_name
                            ),
                            self.line_no(self.pos),
                            1,
                        ));
                    }
                    self.pos += 1;
                    closed = true;
                    break;
                }
            } else if is_any_close(line) {
                self.diagnostics.push(diag(
                    "NODX-E005",
                    "error",
                    "Unmatched block closer.",
                    self.line_no(self.pos),
                    1,
                ));
                self.pos += 1;
                continue;
            }

            if line.trim().is_empty() {
                self.pos += 1;
                continue;
            }
            if is_thematic_break(line) {
                let hr_line = self.line_no(self.pos);
                self.pos += 1;
                if !self.count_node(hr_line) {
                    break;
                }
                out.push(Node::container("hr", Attrs::default(), Vec::new()));
                continue;
            }
            if is_markdown_blockquote_start(line) {
                let quote_line = self.line_no(self.pos);
                if !self.count_node(quote_line) {
                    break;
                }
                out.push(self.parse_markdown_blockquote());
                continue;
            }
            if let Some((colons, name, attrs)) =
                parse_opener_with_cap(line, self.limits.attribute_value_bytes)
            {
                let opener_line = self.line_no(self.pos);
                if !self.count_node(opener_line) {
                    break;
                }
                out.push(self.parse_delimited(colons, name, attrs));
                continue;
            }
            if let Some((level, content, attrs)) =
                parse_heading_with_cap(line, self.limits.attribute_value_bytes)
            {
                let heading_line = self.line_no(self.pos);
                self.pos += 1;
                if !self.count_node(heading_line) {
                    break;
                }
                let mut attr_map = attrs.unwrap_or_default();
                attr_map
                    .attrs
                    .insert("level".to_string(), level.to_string());
                out.push(Node::textual("heading", attr_map, parse_inlines(content)));
                continue;
            }
            if is_list_start(line) {
                let list_line = self.line_no(self.pos);
                if !self.count_node(list_line) {
                    break;
                }
                out.push(self.parse_list());
                continue;
            }
            if self.pos + 1 < self.lines.len()
                && is_pipe_table_header(line, self.lines[self.pos + 1])
            {
                let table_line = self.line_no(self.pos);
                if !self.count_node(table_line) {
                    break;
                }
                out.push(self.parse_pipe_table());
                continue;
            }
            // CommonMark compatibility warnings (W030..W035). These do not
            // alter parse output; they hint did-you-mean for users coming from
            // CommonMark/GFM. See `docs/reference/diagnostics.md` and RFC §23.
            self.emit_block_commonmark_warnings();
            let para_line = self.line_no(self.pos);
            if !self.count_node(para_line) {
                break;
            }
            out.push(self.parse_paragraph());
        }
        if !closed {
            self.diagnostics.push(diag(
                "NODX-E005",
                "error",
                "Unclosed delimited block at end of input.",
                self.line_offset + self.pos.max(1),
                1,
            ));
        }
        out
    }

    fn parse_delimited(&mut self, colons: usize, name: String, attrs: Attrs) -> Node {
        let opener_line = self.line_no(self.pos);
        self.pos += 1;
        self.depth += 1;
        if self.depth > self.limits.block_nesting_depth && !self.depth_limit_hit {
            self.diagnostics.push(diag(
                "NODX-E012",
                "fatal",
                "Block nesting depth limit exceeded.",
                opener_line,
                1,
            ));
            self.depth_limit_hit = true;
        }
        if matches!(name.as_str(), "code" | "pre" | "math" | "style") {
            let start = self.pos;
            while self.pos < self.lines.len()
                && parse_matching_close(self.lines[self.pos], colons, &name).is_none()
            {
                self.pos += 1;
            }
            let text = self.lines[start..self.pos].join("\n");
            if self.pos < self.lines.len() {
                if let Some(Some(label)) = parse_matching_close(self.lines[self.pos], colons, &name)
                    && label != name
                {
                    self.diagnostics.push(diag(
                        "NODX-E005",
                        "error",
                        &format!(
                            "Closing label `{}` does not match open block `{}`.",
                            label, name
                        ),
                        self.line_no(self.pos),
                        1,
                    ));
                }
                self.pos += 1;
            } else {
                self.diagnostics.push(diag(
                    "NODX-E005",
                    "error",
                    "Unclosed literal block.",
                    self.line_no(start),
                    1,
                ));
            }
            self.depth -= 1;
            return Node::literal(&name, attrs, text);
        }

        let children = self.parse_until(Some((colons, name.as_str())));
        self.depth -= 1;
        Node::container(&name, attrs, children)
    }

    fn parse_list(&mut self) -> Node {
        let first = self.lines[self.pos];
        let kind = list_kind(first);
        let mut items = Vec::new();
        while self.pos < self.lines.len() && list_kind(self.lines[self.pos]) == kind {
            let item_line = self.line_no(self.pos);
            let raw = self.lines[self.pos];
            let (content, checked) = strip_list_marker(raw);
            self.pos += 1;
            let mut parts = vec![content.to_string()];
            while self.pos < self.lines.len() && self.lines[self.pos].starts_with("  ") {
                parts.push(self.lines[self.pos].trim_start().to_string());
                self.pos += 1;
            }
            if !self.count_node(item_line) {
                break;
            }
            let mut attrs = Attrs::default();
            if let Some(done) = checked {
                attrs.attrs.insert("checked".to_string(), done.to_string());
            }
            items.push(Node::textual(
                "item",
                attrs,
                parse_inlines(&parts.join("\n")),
            ));
        }
        let mut attrs = Attrs::default();
        attrs
            .attrs
            .insert("kind".to_string(), kind.unwrap_or("unordered").to_string());
        Node::container("list", attrs, items)
    }

    fn parse_pipe_table(&mut self) -> Node {
        let header_line = self.line_no(self.pos);
        let header = split_pipe_row(self.lines[self.pos]);
        let aligns = split_pipe_alignments(self.lines[self.pos + 1]);
        self.pos += 2;
        let mut rows = Vec::new();
        if self.count_node(header_line) {
            rows.push(table_row(header, true, &aligns));
        }
        while self.pos < self.lines.len()
            && self.lines[self.pos].contains('|')
            && !self.lines[self.pos].trim().is_empty()
        {
            let row_line = self.line_no(self.pos);
            if !self.count_node(row_line) {
                break;
            }
            rows.push(table_row(
                split_pipe_row(self.lines[self.pos]),
                false,
                &aligns,
            ));
            self.pos += 1;
        }
        Node::container("table", Attrs::default(), rows)
    }

    fn parse_paragraph(&mut self) -> Node {
        let start = self.pos;
        self.pos += 1;
        while self.pos < self.lines.len()
            && !self.lines[self.pos].trim().is_empty()
            && parse_opener(self.lines[self.pos]).is_none()
            && parse_heading(self.lines[self.pos]).is_none()
            && !is_list_start(self.lines[self.pos])
            && !is_thematic_break(self.lines[self.pos])
            && !is_markdown_blockquote_start(self.lines[self.pos])
            && !is_any_close(self.lines[self.pos])
        {
            if self.pos + 1 < self.lines.len()
                && is_pipe_table_header(self.lines[self.pos], self.lines[self.pos + 1])
            {
                break;
            }
            self.pos += 1;
        }
        // Emit CommonMark inline-shape warnings (W032/W033/W035) on the
        // paragraph slice before turning it into inlines. The scan operates
        // on the raw source lines so JS and Python can replicate it
        // line-for-line and produce byte-identical diagnostics.
        scan_inline_commonmark_warnings(
            &self.lines[start..self.pos],
            self.line_offset + start,
            &mut self.diagnostics,
        );
        Node::textual(
            "paragraph",
            Attrs::default(),
            parse_inlines(&self.lines[start..self.pos].join("\n")),
        )
    }

    /// Parse a Markdown-style block quote (`>` prefix lines) as the syntactic
    /// alias of `::quote`. The resulting AST node is byte-for-byte identical
    /// to the `::quote` form: same `node_type`, default attrs, same children.
    /// The discriminator is purely lexical at parse time; canonical JSON does
    /// not carry any "source" or "kind" hint.
    ///
    /// Lazy continuation is *not* supported (unlike CommonMark): every line of
    /// the quote must start with either `> ` (content) or `>` alone (a blank
    /// line inside the quote). The first line that does not match closes the
    /// block. Inside the body, nested `> ` is parsed recursively, producing a
    /// nested `quote` node.
    fn parse_markdown_blockquote(&mut self) -> Node {
        let opener_line = self.line_no(self.pos);
        let mut stripped: Vec<String> = Vec::new();
        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            if let Some(body) = strip_blockquote_prefix(line) {
                stripped.push(body.to_string());
                self.pos += 1;
            } else {
                break;
            }
        }
        self.depth += 1;
        if self.depth > self.limits.block_nesting_depth && !self.depth_limit_hit {
            self.diagnostics.push(diag(
                "NODX-E012",
                "fatal",
                "Block nesting depth limit exceeded.",
                opener_line,
                1,
            ));
            self.depth_limit_hit = true;
        }
        // Recursively parse the stripped body as a fresh block stream. A
        // dedicated sub-Parser keeps `count_node` / `depth` accounting under
        // the caller's limits, then we drain its diagnostics back so the
        // parent sees them in source order.
        let inner_lines: Vec<&str> = stripped.iter().map(String::as_str).collect();
        let mut sub = Parser {
            lines: &inner_lines,
            pos: 0,
            diagnostics: Vec::new(),
            limits: self.limits,
            node_count: self.node_count,
            line_offset: opener_line - 1,
            depth: self.depth,
            nodes_limit_hit: self.nodes_limit_hit,
            depth_limit_hit: self.depth_limit_hit,
        };
        let children = sub.parse_until(None);
        self.node_count = sub.node_count;
        self.nodes_limit_hit = sub.nodes_limit_hit;
        self.depth_limit_hit = sub.depth_limit_hit;
        self.diagnostics.extend(sub.diagnostics);
        self.depth -= 1;
        Node::container("quote", Attrs::default(), children)
    }

    /// Block-level CommonMark compatibility checks.
    ///
    /// Looks at `self.lines[self.pos]` (and the next line for setext) and
    /// pushes warnings for constructs NODX does not natively support. Never
    /// advances `self.pos`; the regular parse flow handles that.
    fn emit_block_commonmark_warnings(&mut self) {
        let pos = self.pos;
        if pos >= self.lines.len() {
            return;
        }
        let line = self.lines[pos];

        // W030 — Setext-style heading: the *next* line is `===` (≥3) and
        // the current line is a non-empty potential heading text. The `-` /
        // `---` variant is intercepted upstream as a thematic break (PR1)
        // or as a closer/empty line, so we only fire on `=`.
        if !line.trim().is_empty() && pos + 1 < self.lines.len() {
            let next = self.lines[pos + 1];
            if next.len() >= 3 && next.chars().all(|c| c == '=') {
                self.diagnostics.push(diag(
                    "NODX-W030",
                    "warning",
                    "Setext-style heading detected. Use '# Heading' (ATX-style) instead.",
                    self.line_offset + pos + 2,
                    1,
                ));
            }
        }

        // W031 — Indented code block: a line that starts with 4 spaces and
        // is not inside a list item (the surrounding parse loop already
        // dispatches lists separately, so we only see top-level blocks
        // here). One warning per contiguous run; subsequent indented
        // lines are absorbed by `parse_paragraph` without re-emitting.
        if line.starts_with("    ") {
            let prev_indented = pos > 0 && self.lines[pos - 1].starts_with("    ");
            if !prev_indented {
                self.diagnostics.push(diag(
                    "NODX-W031",
                    "warning",
                    "Indented code block detected. Use '::code' fenced block instead.",
                    self.line_no(pos),
                    1,
                ));
            }
        }

        // W034 — GFM footnote definition: an isolated line of the form
        // `[^id]: …`. NODX parses it as a paragraph; the warning points
        // users at the `::footnote` block.
        if is_footnote_definition(line) {
            self.diagnostics.push(diag(
                "NODX-W034",
                "warning",
                "Footnote definitions are not part of 1.0. Use the '::footnote' block.",
                self.line_no(pos),
                1,
            ));
        }
    }
}
/// A thematic break (NODX-RFC-0001 §6) is a line whose trimmed content is
/// three or more repetitions of a single marker char `-`, `*`, or `_` with no
/// internal whitespace. The opening front matter `---` is consumed before
/// `parse_until` runs, so by the time this helper sees a line the document is
/// past the front-matter region and `---` is unambiguously a thematic break.
pub(crate) fn is_thematic_break(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.len() < 3 {
        return false;
    }
    let first = trimmed.as_bytes()[0];
    if !matches!(first, b'-' | b'*' | b'_') {
        return false;
    }
    trimmed.bytes().all(|b| b == first)
}

/// A Markdown-style block quote line starts with either `> ` (greater-than
/// followed by space plus any tail) or `>` alone (blank quote line). `>>`
/// and `>foo` without a separating space are *not* quote markers — they
/// fall through to the paragraph fallback as literal text.
pub(crate) fn is_markdown_blockquote_start(line: &str) -> bool {
    strip_blockquote_prefix(line).is_some()
}

/// Strip a single Markdown blockquote prefix from `line`, returning the
/// remaining body. Returns:
///
/// - `Some("...")` when the line starts with `> ` (the body after the prefix).
/// - `Some("")` when the line is exactly `>` (blank line inside the quote).
/// - `None` otherwise.
///
/// Nested quotes are produced by recursion: a body of `> foo` strips down
/// to `foo` after a second pass.
pub(crate) fn strip_blockquote_prefix(line: &str) -> Option<&str> {
    if let Some(rest) = line.strip_prefix("> ") {
        return Some(rest);
    }
    if line == ">" {
        return Some("");
    }
    None
}

/// Returns `true` when `line` is `[^id]:` followed by space and content,
/// i.e. a GFM footnote definition. We deliberately do not parse the body —
/// the caller emits a warning and lets the regular paragraph flow run.
fn is_footnote_definition(line: &str) -> bool {
    let Some(rest) = line.strip_prefix("[^") else {
        return false;
    };
    let Some(end) = rest.find(']') else {
        return false;
    };
    let id = &rest[..end];
    if id.is_empty()
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return false;
    }
    let after = &rest[end + 1..];
    after.starts_with(':')
}

/// Scan a paragraph slice for CommonMark-only inline shapes and emit
/// W032/W033/W035 warnings. The scan is purely textual and is replicated
/// byte-for-byte in `packages/nodx-js/src/blockParser.mjs` and
/// `packages/nodx-py/src/nodx/block_parser.py` so all three parsers agree
/// on the resulting diagnostics list (order, line, column, message).
fn scan_inline_commonmark_warnings(
    lines: &[&str],
    base_line: usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for (offset, line) in lines.iter().enumerate() {
        let line_no = base_line + offset + 1;

        // W033 — top-level link reference definition `[ref]: url`.
        // Only triggers on the first line of the paragraph; if it appears
        // mid-paragraph it falls back to the inline scan below.
        if offset == 0 && is_link_reference_definition(line) {
            diagnostics.push(diag(
                "NODX-W033",
                "warning",
                "Link reference syntax not supported. Use inline links '[label](url)' instead.",
                line_no,
                1,
            ));
        }

        let bytes = line.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            let rest = &line[i..];

            // W032 — inline image `![alt](url)`.
            if rest.starts_with("![")
                && let Some(close) = rest.find("](")
                && let Some(end) = rest[close + 2..].find(')')
            {
                diagnostics.push(diag(
                    "NODX-W032",
                    "warning",
                    "Inline image syntax not supported in 1.0. Use ':::image' block (see RFC §6) for block-level images.",
                    line_no,
                    i + 1,
                ));
                i += close + 2 + end + 1;
                continue;
            }

            // W033 — inline link reference `[label][ref]` or `[label][]`.
            if rest.starts_with('[')
                && !rest.starts_with("[^")
                && !rest.starts_with("[@")
                && let Some(close) = rest.find(']')
                && rest[close + 1..].starts_with('[')
                && let Some(end) = rest[close + 1..].find(']')
                && close > 0
            {
                diagnostics.push(diag(
                    "NODX-W033",
                    "warning",
                    "Link reference syntax not supported. Use inline links '[label](url)' instead.",
                    line_no,
                    i + 1,
                ));
                i += close + 1 + end + 1;
                continue;
            }

            // W035 — HTML entity reference: `&name;`, `&#NNN;`, `&#xHHH;`.
            if rest.starts_with('&')
                && let Some(len) = html_entity_length(rest)
            {
                diagnostics.push(diag(
                    "NODX-W035",
                    "warning",
                    "HTML entity references are not decoded. Use the Unicode character directly.",
                    line_no,
                    i + 1,
                ));
                i += len;
                continue;
            }

            // Advance one UTF-8 char.
            let ch_len = line[i..].chars().next().map_or(1, char::len_utf8);
            i += ch_len;
        }
    }
}

/// `[name]: target` on a single line, with optional title. Only the
/// minimal CommonMark "link reference definition" shape is matched —
/// just enough to alert the author.
fn is_link_reference_definition(line: &str) -> bool {
    let Some(rest) = line.strip_prefix('[') else {
        return false;
    };
    let Some(end) = rest.find(']') else {
        return false;
    };
    let label = &rest[..end];
    if label.is_empty()
        || label.starts_with('^')
        || label.starts_with('@')
        || !label
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == ' ')
    {
        return false;
    }
    let after = &rest[end + 1..];
    let Some(target) = after.strip_prefix(':') else {
        return false;
    };
    !target.trim().is_empty()
}

/// Return the byte length of a leading HTML entity reference, including
/// the leading `&` and trailing `;`. Returns `None` if the slice does not
/// start with a well-formed entity.
fn html_entity_length(rest: &str) -> Option<usize> {
    let bytes = rest.as_bytes();
    if bytes.first()? != &b'&' {
        return None;
    }
    let mut i = 1;
    if bytes.get(i)? == &b'#' {
        i += 1;
        let hex = bytes.get(i) == Some(&b'x') || bytes.get(i) == Some(&b'X');
        if hex {
            i += 1;
        }
        let digits_start = i;
        while let Some(byte) = bytes.get(i) {
            let ok = if hex {
                byte.is_ascii_hexdigit()
            } else {
                byte.is_ascii_digit()
            };
            if !ok {
                break;
            }
            i += 1;
        }
        if i == digits_start {
            return None;
        }
    } else {
        let name_start = i;
        while let Some(byte) = bytes.get(i) {
            if !byte.is_ascii_alphanumeric() {
                break;
            }
            i += 1;
        }
        if i == name_start {
            return None;
        }
    }
    if bytes.get(i) == Some(&b';') {
        Some(i + 1)
    } else {
        None
    }
}

fn is_any_close(line: &str) -> bool {
    let n = line.chars().take_while(|c| *c == ':').count();
    if n < 2 {
        return false;
    }
    let after = &line[n..];
    if after.trim().is_empty() {
        return true;
    }
    if let Some(rest) = after.strip_prefix(' ') {
        return valid_name(rest.trim_end(), true);
    }
    false
}

fn is_list_start(line: &str) -> bool {
    list_kind(line).is_some()
}

/// Classify the first source line of a potential list. The marker character
/// is normalized away here: the AST records only `unordered`, `ordered`, or
/// `task` so the canonical form is byte-stable regardless of whether the
/// author wrote `- `, `* `, `+ ` (unordered) or `1.`, `1)` (ordered).
fn list_kind(line: &str) -> Option<&'static str> {
    if line.starts_with("- [ ] ") || line.starts_with("- [x] ") {
        return Some("task");
    }
    if line.starts_with("- ") || line.starts_with("* ") || line.starts_with("+ ") {
        return Some("unordered");
    }
    if let Some(rest) = ordered_marker_len(line) {
        let _ = rest;
        return Some("ordered");
    }
    None
}

/// Length of an ordered list marker (`<digits>. ` or `<digits>) `), returning
/// the number of bytes consumed including the trailing space, or `None`. The
/// numeric value of the marker is not preserved in the AST.
fn ordered_marker_len(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i == 0 {
        return None;
    }
    let sep = bytes.get(i).copied()?;
    if sep != b'.' && sep != b')' {
        return None;
    }
    if bytes.get(i + 1).copied()? != b' ' {
        return None;
    }
    Some(i + 2)
}

fn strip_list_marker(line: &str) -> (&str, Option<bool>) {
    if let Some(rest) = line.strip_prefix("- [ ] ") {
        return (rest, Some(false));
    }
    if let Some(rest) = line.strip_prefix("- [x] ") {
        return (rest, Some(true));
    }
    if let Some(rest) = line.strip_prefix("- ") {
        return (rest, None);
    }
    if let Some(rest) = line.strip_prefix("* ") {
        return (rest, None);
    }
    if let Some(rest) = line.strip_prefix("+ ") {
        return (rest, None);
    }
    let consumed = ordered_marker_len(line).expect("ordered marker has been verified by list_kind");
    (&line[consumed..], None)
}

fn is_pipe_table_header(a: &str, b: &str) -> bool {
    a.contains('|')
        && b.trim().chars().all(|c| matches!(c, '|' | '-' | ':' | ' '))
        && b.contains('-')
}

fn split_pipe_row(line: &str) -> Vec<String> {
    line.trim()
        .trim_matches('|')
        .split('|')
        .map(|c| c.trim().to_string())
        .collect()
}

fn split_pipe_alignments(line: &str) -> Vec<Option<String>> {
    split_pipe_row(line)
        .into_iter()
        .map(|cell| {
            let trimmed = cell.trim();
            match (trimmed.starts_with(':'), trimmed.ends_with(':')) {
                (true, true) => Some("center".to_string()),
                (true, false) => Some("left".to_string()),
                (false, true) => Some("right".to_string()),
                (false, false) => None,
            }
        })
        .collect()
}

fn table_row(cells: Vec<String>, header: bool, aligns: &[Option<String>]) -> Node {
    let children = cells
        .into_iter()
        .enumerate()
        .map(|(index, cell)| {
            let (mut attrs, content) = parse_pipe_cell_attrs(&cell);
            if header {
                attrs.attrs.insert("header".to_string(), "true".to_string());
                attrs.attrs.insert("scope".to_string(), "col".to_string());
            }
            if let Some(Some(align)) = aligns.get(index) {
                attrs
                    .attrs
                    .entry("align".to_string())
                    .or_insert_with(|| align.clone());
            }
            Node::textual("cell", attrs, parse_inlines(content))
        })
        .collect();
    Node::container("row", Attrs::default(), children)
}

fn parse_pipe_cell_attrs(cell: &str) -> (Attrs, &str) {
    let trimmed = cell.trim_start();
    if !trimmed.starts_with('{') {
        return (Attrs::default(), cell);
    }
    let Some(end) = trimmed.find('}') else {
        return (Attrs::default(), cell);
    };
    let raw_attrs = &trimmed[..=end];
    let after = &trimmed[end + 1..];
    if !(after.is_empty() || after.starts_with(' ')) {
        return (Attrs::default(), cell);
    }
    match parse_attrs(raw_attrs) {
        Some(attrs) if !attrs_is_empty(&attrs) => (attrs, after.trim_start()),
        None => (Attrs::default(), cell),
        Some(_) => (Attrs::default(), cell),
    }
}

fn attrs_is_empty(attrs: &Attrs) -> bool {
    attrs.id.is_none()
        && attrs.classes.is_empty()
        && attrs.attrs.is_empty()
        && attrs.styles.is_empty()
}

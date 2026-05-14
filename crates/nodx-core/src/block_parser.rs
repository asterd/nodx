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
    depth: usize,
    nodes_limit_hit: bool,
    depth_limit_hit: bool,
}

impl Parser<'_> {
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
                    self.pos + 1,
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
                            self.pos + 1,
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
                    self.pos + 1,
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
                let hr_line = self.pos + 1;
                self.pos += 1;
                if !self.count_node(hr_line) {
                    break;
                }
                out.push(Node::container("hr", Attrs::default(), Vec::new()));
                continue;
            }
            if let Some((colons, name, attrs)) =
                parse_opener_with_cap(line, self.limits.attribute_value_bytes)
            {
                let opener_line = self.pos + 1;
                if !self.count_node(opener_line) {
                    break;
                }
                out.push(self.parse_delimited(colons, name, attrs));
                continue;
            }
            if let Some((level, content, attrs)) =
                parse_heading_with_cap(line, self.limits.attribute_value_bytes)
            {
                let heading_line = self.pos + 1;
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
                let list_line = self.pos + 1;
                if !self.count_node(list_line) {
                    break;
                }
                out.push(self.parse_list());
                continue;
            }
            if self.pos + 1 < self.lines.len()
                && is_pipe_table_header(line, self.lines[self.pos + 1])
            {
                let table_line = self.pos + 1;
                if !self.count_node(table_line) {
                    break;
                }
                out.push(self.parse_pipe_table());
                continue;
            }
            let para_line = self.pos + 1;
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
                self.pos.max(1),
                1,
            ));
        }
        out
    }

    fn parse_delimited(&mut self, colons: usize, name: String, attrs: Attrs) -> Node {
        let opener_line = self.pos + 1;
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
                        self.pos + 1,
                        1,
                    ));
                }
                self.pos += 1;
            } else {
                self.diagnostics.push(diag(
                    "NODX-E005",
                    "error",
                    "Unclosed literal block.",
                    start + 1,
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
            let item_line = self.pos + 1;
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
        let header_line = self.pos + 1;
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
            let row_line = self.pos + 1;
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
            && !is_any_close(self.lines[self.pos])
        {
            if self.pos + 1 < self.lines.len()
                && is_pipe_table_header(self.lines[self.pos], self.lines[self.pos + 1])
            {
                break;
            }
            self.pos += 1;
        }
        Node::textual(
            "paragraph",
            Attrs::default(),
            parse_inlines(&self.lines[start..self.pos].join("\n")),
        )
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

fn list_kind(line: &str) -> Option<&'static str> {
    if line.starts_with("- [ ] ") || line.starts_with("- [x] ") {
        Some("task")
    } else if line.starts_with("- ") {
        Some("unordered")
    } else {
        let (n, _) = line.split_once(". ")?;
        if !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()) {
            Some("ordered")
        } else {
            None
        }
    }
}

fn strip_list_marker(line: &str) -> (&str, Option<bool>) {
    if let Some(rest) = line.strip_prefix("- [ ] ") {
        (rest, Some(false))
    } else if let Some(rest) = line.strip_prefix("- [x] ") {
        (rest, Some(true))
    } else if let Some(rest) = line.strip_prefix("- ") {
        (rest, None)
    } else {
        let (_, rest) = line.split_once(". ").unwrap();
        (rest, None)
    }
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

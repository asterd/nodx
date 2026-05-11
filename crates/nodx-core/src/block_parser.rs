use std::collections::BTreeMap;

use crate::ResourceLimits;
use crate::ast::{Attrs, Document, Node, Value};
use crate::attrs::{parse_close, parse_heading, parse_opener, valid_name};
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

    let normalized = input.replace("\r\n", "\n");
    let mut lines: Vec<&str> = normalized.split('\n').collect();
    if lines.last() == Some(&"") {
        lines.pop();
    }

    let mut meta = BTreeMap::new();
    let mut start = 0;
    if lines.first() == Some(&"---") {
        match lines.iter().skip(1).position(|line| *line == "---") {
            Some(end_rel) => {
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
    meta.entry("schema".to_string())
        .or_insert(Value::String("nodx/1.0".to_string()));
    meta.entry("type".to_string())
        .or_insert(Value::String("document".to_string()));
    meta.entry("dir".to_string())
        .or_insert(Value::String("auto".to_string()));
    meta.entry("language".to_string())
        .or_insert(Value::String("und".to_string()));

    let mut parser = Parser {
        lines: &lines,
        pos: start,
        diagnostics,
        limits,
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

struct Parser<'a> {
    lines: &'a [&'a str],
    pos: usize,
    diagnostics: Vec<Diagnostic>,
    limits: ResourceLimits,
}

impl Parser<'_> {
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
                if let Some(label) = parse_close(line, n) {
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
            if let Some((colons, name, attrs)) = parse_opener(line) {
                out.push(self.parse_delimited(colons, name, attrs));
                continue;
            }
            if let Some((level, content, attrs)) = parse_heading(line) {
                self.pos += 1;
                let mut attr_map = attrs.unwrap_or_default();
                attr_map
                    .attrs
                    .insert("level".to_string(), level.to_string());
                out.push(Node::textual("heading", attr_map, parse_inlines(content)));
                continue;
            }
            if is_list_start(line) {
                out.push(self.parse_list());
                continue;
            }
            if self.pos + 1 < self.lines.len()
                && is_pipe_table_header(line, self.lines[self.pos + 1])
            {
                out.push(self.parse_pipe_table());
                continue;
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
        self.pos += 1;
        if matches!(name.as_str(), "code" | "pre" | "math" | "style") {
            let start = self.pos;
            while self.pos < self.lines.len() && parse_close(self.lines[self.pos], colons).is_none()
            {
                self.pos += 1;
            }
            let text = self.lines[start..self.pos].join("\n");
            if self.pos < self.lines.len() {
                if let Some(Some(label)) = parse_close(self.lines[self.pos], colons)
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
            return Node::literal(&name, attrs, text);
        }

        let children = self.parse_until(Some((colons, name.as_str())));
        Node::container(&name, attrs, children)
    }

    fn parse_list(&mut self) -> Node {
        let first = self.lines[self.pos];
        let kind = list_kind(first);
        let mut items = Vec::new();
        while self.pos < self.lines.len() && list_kind(self.lines[self.pos]) == kind {
            let raw = self.lines[self.pos];
            let (content, checked) = strip_list_marker(raw);
            self.pos += 1;
            let mut parts = vec![content.to_string()];
            while self.pos < self.lines.len() && self.lines[self.pos].starts_with("  ") {
                parts.push(self.lines[self.pos].trim_start().to_string());
                self.pos += 1;
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
        let header = split_pipe_row(self.lines[self.pos]);
        self.pos += 2;
        let mut rows = vec![table_row(header, true)];
        while self.pos < self.lines.len()
            && self.lines[self.pos].contains('|')
            && !self.lines[self.pos].trim().is_empty()
        {
            rows.push(table_row(split_pipe_row(self.lines[self.pos]), false));
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
fn is_any_close(line: &str) -> bool {
    let n = line.chars().take_while(|c| *c == ':').count();
    if n < 3 {
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

fn table_row(cells: Vec<String>, header: bool) -> Node {
    let children = cells
        .into_iter()
        .map(|cell| {
            let mut attrs = Attrs::default();
            if header {
                attrs.attrs.insert("header".to_string(), "true".to_string());
                attrs.attrs.insert("scope".to_string(), "col".to_string());
            }
            Node::textual("cell", attrs, parse_inlines(&cell))
        })
        .collect();
    Node::container("row", Attrs::default(), children)
}

use std::collections::{BTreeMap, BTreeSet};

use crate::ast::Value;
use crate::diagnostic::{Diagnostic, diag};

pub(crate) fn parse_front_matter(
    lines: &[&str],
    diagnostics: &mut Vec<Diagnostic>,
) -> BTreeMap<String, Value> {
    let events = yaml_events(lines, diagnostics);
    let mut parser = EventParser {
        events: &events,
        pos: 0,
    };
    parser.parse_mapping(0, diagnostics)
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum YamlEvent {
    Mapping {
        indent: usize,
        line: usize,
        key: String,
        value: Option<String>,
    },
    Sequence {
        indent: usize,
        line: usize,
        value: String,
    },
    Scalar {
        indent: usize,
        text: String,
    },
}

fn yaml_events(lines: &[&str], diagnostics: &mut Vec<Diagnostic>) -> Vec<YamlEvent> {
    let mut events = Vec::new();
    let mut block_scalar_indent: Option<usize> = None;
    for (idx, line) in lines.iter().enumerate() {
        let line_no = idx + 2;
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let indent = line.chars().take_while(|c| *c == ' ').count();
        let trimmed = &line[indent..];
        if let Some(block_indent) = block_scalar_indent {
            if indent >= block_indent {
                // Inside a block scalar, content is opaque text — do not apply
                // mapping/sequence safety heuristics or we will reject literal
                // `&`/`*`/`!` characters that are valid scalar content.
                events.push(YamlEvent::Scalar {
                    indent,
                    text: line[block_indent.min(line.len())..].to_string(),
                });
                continue;
            }
            block_scalar_indent = None;
        }
        check_yaml_safety(line, line_no, diagnostics);
        if let Some(rest) = trimmed.strip_prefix("- ") {
            events.push(YamlEvent::Sequence {
                indent,
                line: line_no,
                value: rest.trim().to_string(),
            });
        } else if trimmed == "-" {
            events.push(YamlEvent::Sequence {
                indent,
                line: line_no,
                value: String::new(),
            });
        } else if let Some((key, value)) = split_mapping(trimmed) {
            let key = unquote(key.trim());
            let value = value.trim();
            if is_block_scalar(value) {
                block_scalar_indent = Some(indent + 2);
            }
            events.push(YamlEvent::Mapping {
                indent,
                line: line_no,
                key,
                value: if value.is_empty() {
                    None
                } else {
                    Some(value.to_string())
                },
            });
        } else {
            diagnostics.push(forbidden(line_no, "Forbidden YAML safe-subset construct."));
        }
    }
    events
}

struct EventParser<'a> {
    events: &'a [YamlEvent],
    pos: usize,
}

impl EventParser<'_> {
    fn parse_mapping(
        &mut self,
        indent: usize,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> BTreeMap<String, Value> {
        let mut map = BTreeMap::new();
        let mut seen = BTreeSet::new();
        while let Some(YamlEvent::Mapping {
            indent: item_indent,
            line,
            key,
            value,
        }) = self.events.get(self.pos)
        {
            if *item_indent < indent {
                break;
            }
            if *item_indent > indent {
                self.pos += 1;
                continue;
            }
            if !seen.insert(key.clone()) {
                diagnostics.push(forbidden(*line, "Duplicate front matter key."));
            }
            if key == "<<" {
                diagnostics.push(forbidden(*line, "YAML merge keys are not supported."));
            }
            self.pos += 1;
            let parsed = match value {
                Some(raw) if is_block_scalar(raw) => self.parse_block_scalar(*item_indent),
                Some(raw) => scalar(raw),
                None => self.parse_nested_value(*item_indent, diagnostics),
            };
            map.insert(key.clone(), parsed);
        }
        map
    }

    fn parse_sequence(&mut self, indent: usize, diagnostics: &mut Vec<Diagnostic>) -> Vec<Value> {
        let mut out = Vec::new();
        while let Some(YamlEvent::Sequence {
            indent: item_indent,
            line,
            value,
        }) = self.events.get(self.pos)
        {
            if *item_indent != indent {
                break;
            }
            self.pos += 1;
            if value.is_empty() {
                out.push(self.parse_nested_value(*item_indent, diagnostics));
            } else if let Some((key, rest)) = split_mapping(value) {
                let mut child = BTreeMap::new();
                let key = unquote(key.trim());
                child.insert(
                    key,
                    if rest.trim().is_empty() {
                        self.parse_nested_value(*item_indent, diagnostics)
                    } else {
                        scalar(rest.trim())
                    },
                );
                if matches!(
                    self.events.get(self.pos),
                    Some(YamlEvent::Mapping { indent: next, .. }) if *next > *item_indent
                ) {
                    let nested = self.parse_mapping(*item_indent + 2, diagnostics);
                    for key in nested.keys() {
                        if child.contains_key(key) {
                            diagnostics.push(forbidden(*line, "Duplicate front matter key."));
                        }
                    }
                    child.extend(nested);
                }
                out.push(Value::Map(child));
            } else {
                out.push(scalar(value));
            }
        }
        out
    }

    fn parse_nested_value(
        &mut self,
        parent_indent: usize,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Value {
        let Some(next) = self.events.get(self.pos) else {
            return Value::Map(BTreeMap::new());
        };
        match next {
            YamlEvent::Sequence { indent, .. } if *indent > parent_indent => {
                Value::List(self.parse_sequence(*indent, diagnostics))
            }
            YamlEvent::Mapping { indent, .. } if *indent > parent_indent => {
                Value::Map(self.parse_mapping(*indent, diagnostics))
            }
            _ => Value::Map(BTreeMap::new()),
        }
    }

    fn parse_block_scalar(&mut self, parent_indent: usize) -> Value {
        let mut parts = Vec::new();
        while let Some(event) = self.events.get(self.pos) {
            match event {
                YamlEvent::Scalar { indent, text } if *indent > parent_indent => {
                    parts.push(text.clone());
                }
                YamlEvent::Mapping {
                    indent, key, value, ..
                } if *indent > parent_indent => {
                    if let Some(value) = value {
                        parts.push(format!("{key}: {value}"));
                    } else {
                        parts.push(format!("{key}:"));
                    }
                }
                _ => break,
            }
            self.pos += 1;
        }
        Value::String(parts.join("\n"))
    }
}

fn check_yaml_safety(line: &str, line_no: usize, diagnostics: &mut Vec<Diagnostic>) {
    let trimmed = line.trim_start();
    let unquoted = unquoted_view(trimmed);
    if trimmed == "---"
        || trimmed == "..."
        || trimmed.starts_with("--- ")
        || trimmed.starts_with("... ")
    {
        diagnostics.push(forbidden(
            line_no,
            "Multiple YAML documents are not supported.",
        ));
    }
    if line
        .chars()
        .take_while(|c| c.is_ascii_whitespace())
        .any(|c| c == '\t')
    {
        diagnostics.push(forbidden(line_no, "Forbidden YAML indentation."));
    }
    if line
        .chars()
        .any(|c| ((c as u32) < 0x20 && c != '\t' && c != '\n' && c != '\r') || c == '\u{007f}')
    {
        diagnostics.push(forbidden(
            line_no,
            "Forbidden control character in YAML front matter.",
        ));
    }
    if unquoted.contains('&') || unquoted.contains('*') || unquoted.contains('!') {
        diagnostics.push(forbidden(line_no, "Forbidden YAML safe-subset construct."));
    }
    if unquoted.contains("<<:") || trimmed.starts_with("? ") {
        diagnostics.push(forbidden(line_no, "Forbidden YAML safe-subset construct."));
    }
    if let Some((key, value)) = split_mapping(trimmed) {
        let key = key.trim();
        if key.is_empty()
            || key.starts_with('[')
            || key.starts_with('{')
            || key == "?"
            || key == "<<"
        {
            diagnostics.push(forbidden(line_no, "Forbidden YAML mapping key."));
        }
        let value_trimmed = value.trim();
        if value_trimmed.starts_with('[') || value_trimmed.starts_with('{') {
            diagnostics.push(forbidden(
                line_no,
                "Flow-style YAML collections are not allowed.",
            ));
        }
        check_scalar_safety(value_trimmed, line_no, diagnostics);
    } else if let Some(value) = trimmed.strip_prefix("- ") {
        let value_trimmed = value.trim();
        if value_trimmed.starts_with('[') || value_trimmed.starts_with('{') {
            diagnostics.push(forbidden(
                line_no,
                "Flow-style YAML collections are not allowed.",
            ));
        }
        check_scalar_safety(value_trimmed, line_no, diagnostics);
    }
}

fn check_scalar_safety(raw: &str, line_no: usize, diagnostics: &mut Vec<Diagnostic>) {
    if raw.is_empty() || is_quoted(raw) || is_block_scalar(raw) {
        return;
    }
    let lower = raw.to_ascii_lowercase();
    if matches!(
        lower.as_str(),
        ".nan" | ".inf" | "+.inf" | "-.inf" | ".infinity" | "+.infinity" | "-.infinity"
    ) {
        diagnostics.push(forbidden(line_no, "Forbidden YAML non-finite number."));
    }
    if lower.starts_with("0x") || lower.starts_with("+0x") || lower.starts_with("-0x") {
        diagnostics.push(forbidden(line_no, "Forbidden YAML numeric special."));
    }
    if lower.starts_with("0b") || lower.starts_with("+0b") || lower.starts_with("-0b") {
        diagnostics.push(forbidden(line_no, "Forbidden YAML numeric special."));
    }
    if lower.starts_with("0o") || lower.starts_with("+0o") || lower.starts_with("-0o") {
        diagnostics.push(forbidden(line_no, "Forbidden YAML numeric special."));
    }
    if matches!(
        lower.as_str(),
        "yes" | "no" | "on" | "off" | "y" | "n"
    ) {
        diagnostics.push(forbidden(
            line_no,
            "Forbidden YAML boolean alias; use true/false.",
        ));
    }
    if is_native_timestamp(raw) {
        diagnostics.push(forbidden(
            line_no,
            "Native YAML timestamps are not supported.",
        ));
    }
}

fn split_mapping(input: &str) -> Option<(&str, &str)> {
    let mut quote = None;
    for (idx, ch) in input.char_indices() {
        match (quote, ch) {
            (Some(q), c) if c == q => quote = None,
            (None, '"' | '\'') => quote = Some(ch),
            (None, ':') => return Some((&input[..idx], &input[idx + 1..])),
            _ => {}
        }
    }
    None
}

fn scalar(raw: &str) -> Value {
    if raw == "null" {
        Value::Null
    } else if raw == "true" {
        Value::Bool(true)
    } else if raw == "false" {
        Value::Bool(false)
    } else if raw.starts_with('[') && raw.ends_with(']') {
        let inner = &raw[1..raw.len() - 1];
        Value::List(
            inner
                .split(',')
                .filter(|s| !s.trim().is_empty())
                .map(|s| scalar(s.trim()))
                .collect(),
        )
    } else if let Some(number) = canonical_number(raw) {
        Value::Number(number)
    } else {
        Value::String(unquote(raw))
    }
}

fn canonical_number(raw: &str) -> Option<String> {
    if !raw.chars().any(|c| c.is_ascii_digit()) {
        return None;
    }
    let value = raw.parse::<f64>().ok()?;
    if !value.is_finite() {
        return None;
    }
    Some(value.to_string())
}

fn unquoted_view(input: &str) -> String {
    let mut out = String::new();
    let mut quote = None;
    for ch in input.chars() {
        match (quote, ch) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(ch),
            (None, c) => out.push(c),
        }
    }
    out
}

fn is_quoted(raw: &str) -> bool {
    (raw.starts_with('"') && raw.ends_with('"')) || (raw.starts_with('\'') && raw.ends_with('\''))
}

fn is_block_scalar(raw: &str) -> bool {
    raw == "|"
        || raw == ">"
        || raw.starts_with("|+")
        || raw.starts_with("|-")
        || raw.starts_with(">+")
        || raw.starts_with(">-")
}

fn is_native_timestamp(raw: &str) -> bool {
    let bytes = raw.as_bytes();
    bytes.len() >= 10
        && bytes[0..4].iter().all(u8::is_ascii_digit)
        && bytes[4] == b'-'
        && bytes[5..7].iter().all(u8::is_ascii_digit)
        && bytes[7] == b'-'
        && bytes[8..10].iter().all(u8::is_ascii_digit)
}

fn forbidden(line: usize, message: &str) -> Diagnostic {
    diag("NODX-E019", "fatal", message, line, 1)
}

pub(crate) fn unquote(raw: &str) -> String {
    if is_quoted(raw) {
        raw[1..raw.len() - 1].to_string()
    } else {
        raw.to_string()
    }
}

use std::collections::BTreeMap;

use crate::ast::Value;
use crate::diagnostic::{Diagnostic, diag};

pub(crate) fn parse_front_matter(
    lines: &[&str],
    diagnostics: &mut Vec<Diagnostic>,
) -> BTreeMap<String, Value> {
    let mut map = BTreeMap::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if line.trim().is_empty() {
            i += 1;
            continue;
        }
        check_yaml_safety(line, i + 2, diagnostics);
        if !line.starts_with(' ') {
            if let Some((k, v)) = line.split_once(':') {
                let key = k.trim().to_string();
                if map.contains_key(&key) {
                    diagnostics.push(diag(
                        "NODX-E020",
                        "fatal",
                        "Duplicate front matter key.",
                        i + 2,
                        1,
                    ));
                }
                let rest = v.trim();
                if rest.is_empty() {
                    let next = lines.get(i + 1).copied().unwrap_or("");
                    if next.trim_start().starts_with("- ") || next.trim() == "-" {
                        let (list, consumed) = parse_block_sequence(lines, i + 1, diagnostics);
                        map.insert(key, Value::List(list));
                        i = consumed;
                        continue;
                    }
                    let (child, consumed) = parse_block_mapping(lines, i + 1, diagnostics);
                    map.insert(key, Value::Map(child));
                    i = consumed;
                    continue;
                }
                map.insert(key, scalar(rest));
            }
        }
        i += 1;
    }
    map
}

fn check_yaml_safety(line: &str, line_no: usize, diagnostics: &mut Vec<Diagnostic>) {
    let trimmed = line.trim_start();
    if trimmed.starts_with('&')
        || trimmed.starts_with("*")
        || trimmed.starts_with("!!")
        || trimmed.starts_with("---")
        || trimmed.starts_with("<<:")
    {
        diagnostics.push(diag(
            "NODX-E019",
            "fatal",
            "Forbidden YAML safe-subset construct.",
            line_no,
            1,
        ));
    }
    if trimmed.contains(": &")
        || trimmed.contains(": *")
        || trimmed.contains(": !!")
        || trimmed.contains("<<:")
        || trimmed.eq("...")
    {
        diagnostics.push(diag(
            "NODX-E019",
            "fatal",
            "Forbidden YAML safe-subset construct.",
            line_no,
            1,
        ));
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.contains(".nan") || lower.contains(".inf") || lower.contains("infinity") {
        diagnostics.push(diag(
            "NODX-E019",
            "fatal",
            "Forbidden YAML non-finite number.",
            line_no,
            1,
        ));
    }
}

fn parse_block_mapping(
    lines: &[&str],
    start: usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> (BTreeMap<String, Value>, usize) {
    let mut map = BTreeMap::new();
    let mut i = start;
    while i < lines.len() && lines[i].starts_with("  ") && !lines[i].trim_start().starts_with("- ")
    {
        check_yaml_safety(lines[i], i + 2, diagnostics);
        if let Some((ck, cv)) = lines[i].trim().split_once(':') {
            map.insert(ck.trim().to_string(), scalar(cv.trim()));
        }
        i += 1;
    }
    (map, i)
}

fn parse_block_sequence(
    lines: &[&str],
    start: usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> (Vec<Value>, usize) {
    let mut out = Vec::new();
    let mut i = start;
    while i < lines.len() {
        let line = lines[i];
        if !line.starts_with("  ") {
            break;
        }
        let trimmed = line.trim_start();
        if !trimmed.starts_with("- ") && trimmed != "-" {
            break;
        }
        check_yaml_safety(line, i + 2, diagnostics);
        let after = if trimmed == "-" { "" } else { &trimmed[2..] };
        if let Some((k, v)) = after.split_once(':') {
            let mut child = BTreeMap::new();
            let key = k.trim().to_string();
            let rest = v.trim();
            if rest.is_empty() {
                i += 1;
                while i < lines.len() && lines[i].starts_with("    ") {
                    check_yaml_safety(lines[i], i + 2, diagnostics);
                    if let Some((ck, cv)) = lines[i].trim().split_once(':') {
                        child.insert(ck.trim().to_string(), scalar(cv.trim()));
                    }
                    i += 1;
                }
                out.push(Value::Map(child));
                continue;
            }
            child.insert(key, scalar(rest));
            i += 1;
            while i < lines.len()
                && lines[i].starts_with("    ")
                && !lines[i].trim_start().starts_with("- ")
            {
                check_yaml_safety(lines[i], i + 2, diagnostics);
                if let Some((ck, cv)) = lines[i].trim().split_once(':') {
                    child.insert(ck.trim().to_string(), scalar(cv.trim()));
                }
                i += 1;
            }
            out.push(Value::Map(child));
        } else {
            out.push(scalar(after));
            i += 1;
        }
    }
    (out, i)
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
        Value::List(inner.split(',').map(|s| scalar(s.trim())).collect())
    } else if raw.parse::<f64>().is_ok() && raw.chars().any(|c| c.is_ascii_digit()) {
        Value::Number(raw.to_string())
    } else {
        Value::String(unquote(raw))
    }
}

pub(crate) fn unquote(raw: &str) -> String {
    if (raw.starts_with('"') && raw.ends_with('"'))
        || (raw.starts_with('\'') && raw.ends_with('\''))
    {
        raw[1..raw.len() - 1].to_string()
    } else {
        raw.to_string()
    }
}

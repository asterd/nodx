use std::collections::BTreeSet;

use crate::ast::Attrs;
use crate::front_matter::unquote;

pub(crate) fn parse_opener(line: &str) -> Option<(usize, String, Attrs)> {
    parse_opener_with_cap(line, 64 * 1024)
}

pub(crate) fn parse_opener_with_cap(
    line: &str,
    value_cap: usize,
) -> Option<(usize, String, Attrs)> {
    let colons = line.chars().take_while(|c| *c == ':').count();
    if colons < 2 {
        return None;
    }
    let rest = &line[colons..];
    let mut parts = rest.splitn(2, ' ');
    let name = parts.next()?.trim();
    if name.is_empty() || !valid_name(name, true) {
        return None;
    }
    let attrs = parts
        .next()
        .and_then(|raw| parse_attrs_with_cap(raw, value_cap))
        .unwrap_or_default();
    Some((colons, name.to_string(), attrs))
}

pub(crate) fn parse_heading(line: &str) -> Option<(usize, &str, Option<Attrs>)> {
    parse_heading_with_cap(line, 64 * 1024)
}

pub(crate) fn parse_heading_with_cap(
    line: &str,
    value_cap: usize,
) -> Option<(usize, &str, Option<Attrs>)> {
    let level = line.chars().take_while(|c| *c == '#').count();
    if !(1..=6).contains(&level) || !line[level..].starts_with(' ') {
        return None;
    }
    let raw = line[level + 1..].trim_end();
    if let Some(pos) = raw.rfind(" {")
        && raw.ends_with('}')
    {
        return Some((
            level,
            raw[..pos].trim_end(),
            parse_attrs_with_cap(&raw[pos + 1..], value_cap),
        ));
    }
    if let Some(pos) = raw.rfind(' ') {
        let candidate = &raw[pos + 1..];
        if let Some(id) = candidate.strip_prefix('#')
            && valid_name(id, true)
            && !raw[..pos].ends_with('\\')
        {
            let attrs = Attrs {
                id: Some(id.to_string()),
                ..Attrs::default()
            };
            return Some((level, raw[..pos].trim_end(), Some(attrs)));
        }
    }
    Some((level, raw, None))
}

pub(crate) fn parse_attrs(raw: &str) -> Option<Attrs> {
    parse_attrs_with_cap(raw, 64 * 1024)
}

pub(crate) fn parse_attrs_with_cap(raw: &str, value_cap: usize) -> Option<Attrs> {
    let s = raw.trim();
    if !s.starts_with('{') || !s.ends_with('}') {
        return None;
    }
    let mut attrs = Attrs::default();
    for token in split_attr_tokens(&s[1..s.len() - 1]) {
        if let Some(id) = token.strip_prefix('#') {
            attrs.id = Some(id.to_string());
        } else if let Some(class) = token.strip_prefix('.') {
            attrs.classes.push(class.to_string());
        } else if let Some((k, v)) = token.split_once('=') {
            let value = unquote(v);
            if value.len() > value_cap {
                continue;
            }
            apply_attr(&mut attrs, k, &value);
        } else if token == "highlight" {
            attrs.styles.insert(
                "background-color".to_string(),
                "color-mix(in srgb, var(--nodx-color-accent) 14%, transparent)".to_string(),
            );
            attrs
                .styles
                .insert("padding".to_string(), "0.05em 0.25em".to_string());
            attrs
                .styles
                .insert("border-radius".to_string(), "0.2em".to_string());
        }
    }
    let mut set = BTreeSet::new();
    attrs.classes.retain(|c| set.insert(c.clone()));
    attrs.classes.sort();
    Some(attrs)
}

pub(crate) fn merge_class_suffix(attrs: &mut Attrs, suffix: &str) {
    let mut rest = suffix;
    while let Some(after_dot) = rest.strip_prefix('.') {
        let len = after_dot
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
            .map(char::len_utf8)
            .sum::<usize>();
        if len == 0 {
            break;
        }
        let class = &after_dot[..len];
        if valid_ident(class) {
            attrs.classes.push(class.to_string());
        }
        rest = &after_dot[len..];
    }
    let mut set = BTreeSet::new();
    attrs.classes.retain(|c| set.insert(c.clone()));
    attrs.classes.sort();
}

fn apply_attr(attrs: &mut Attrs, key: &str, value: &str) {
    if key == "class" {
        attrs
            .classes
            .extend(value.split_whitespace().map(ToString::to_string));
        return;
    }
    if let Some(prop) = inline_style_property(key)
        && safe_inline_style_value(value)
    {
        attrs.styles.insert(prop.to_string(), value.to_string());
        return;
    }
    attrs.attrs.insert(key.to_string(), value.to_string());
}

fn inline_style_property(key: &str) -> Option<&'static str> {
    match key {
        "bg" => Some("background-color"),
        "color" => Some("color"),
        "border" => Some("border"),
        "radius" => Some("border-radius"),
        "margin" => Some("margin"),
        "m" => Some("margin"),
        "gap" => Some("gap"),
        "width" => Some("width"),
        "height" => Some("height"),
        "display" => Some("display"),
        "columns" => Some("grid-template-columns"),
        "text-align" => Some("text-align"),
        "pad" | "padding" => Some("padding"),
        "font" => Some("font"),
        "weight" => Some("font-weight"),
        "background-color" => Some("background-color"),
        "border-radius" => Some("border-radius"),
        "font-weight" => Some("font-weight"),
        "grid-template-columns" => Some("grid-template-columns"),
        _ => None,
    }
}

fn safe_inline_style_value(value: &str) -> bool {
    if value.len() > 240
        || value
            .chars()
            .any(|c| matches!(c, '<' | '>' | '{' | '}' | ';'))
    {
        return false;
    }
    let lower = value.to_ascii_lowercase();
    !["expression(", "javascript:", "vbscript:", "@import", "url("]
        .iter()
        .any(|needle| lower.contains(needle))
}

fn valid_ident(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn split_attr_tokens(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut quoted = false;
    for ch in input.chars() {
        match ch {
            '"' => {
                quoted = !quoted;
                buf.push(ch);
            }
            ' ' if !quoted => {
                if !buf.is_empty() {
                    out.push(std::mem::take(&mut buf));
                }
            }
            _ => buf.push(ch),
        }
    }
    if !buf.is_empty() {
        out.push(buf);
    }
    out
}

pub fn valid_name(s: &str, allow_hyphen: bool) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || (allow_hyphen && c == '-'))
}

pub(crate) fn parse_close(line: &str, n: usize) -> Option<Option<String>> {
    let colons = line.chars().take_while(|c| *c == ':').count();
    if colons != n {
        return None;
    }
    let after = &line[n..];
    if after.trim().is_empty() {
        return Some(None);
    }
    if let Some(rest) = after.strip_prefix(' ') {
        let trimmed = rest.trim_end();
        if valid_name(trimmed, true) {
            return Some(Some(trimmed.to_string()));
        }
    }
    None
}

pub(crate) fn parse_matching_close(
    line: &str,
    n: usize,
    expected_name: &str,
) -> Option<Option<String>> {
    if let Some(close) = parse_close(line, n) {
        return Some(close);
    }
    let after = line.strip_prefix(&":".repeat(n))?;
    if after == expected_name {
        return Some(Some(expected_name.to_string()));
    }
    None
}

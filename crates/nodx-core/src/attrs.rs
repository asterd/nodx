use std::collections::BTreeSet;

use crate::ast::Attrs;
use crate::front_matter::unquote;

pub(crate) fn parse_opener(line: &str) -> Option<(usize, String, Attrs)> {
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
    let attrs = parts.next().and_then(parse_attrs).unwrap_or_default();
    Some((colons, name.to_string(), attrs))
}

pub(crate) fn parse_heading(line: &str) -> Option<(usize, &str, Option<Attrs>)> {
    let level = line.chars().take_while(|c| *c == '#').count();
    if !(1..=6).contains(&level) || !line[level..].starts_with(' ') {
        return None;
    }
    let raw = line[level + 1..].trim_end();
    if let Some(pos) = raw.rfind(" {")
        && raw.ends_with('}')
    {
        return Some((level, raw[..pos].trim_end(), parse_attrs(&raw[pos + 1..])));
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
            attrs.attrs.insert(k.to_string(), unquote(v));
        }
    }
    let mut set = BTreeSet::new();
    attrs.classes.retain(|c| set.insert(c.clone()));
    attrs.classes.sort();
    Some(attrs)
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

pub(crate) fn parse_matching_close(line: &str, n: usize, expected_name: &str) -> Option<Option<String>> {
    if let Some(close) = parse_close(line, n) {
        return Some(close);
    }
    let after = line.strip_prefix(&":".repeat(n))?;
    if after == expected_name {
        return Some(Some(expected_name.to_string()));
    }
    None
}

use crate::ast::{Attrs, Inline, Node};
use crate::attrs::{merge_class_suffix, parse_attrs};

pub fn parse_inlines(input: &str) -> Vec<Inline> {
    let mut out = Vec::new();
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < input.len() {
        let rest = &input[i..];
        // Code span: N backticks open, N backticks close (CommonMark-style).
        // Trim a single leading/trailing space if both sides have one and the
        // content is not all-whitespace. Matches CommonMark "code span" rule.
        if rest.starts_with('`') {
            let run = rest.bytes().take_while(|b| *b == b'`').count();
            if let Some(close_off) = find_backtick_run(rest, run, run) {
                let raw = &rest[run..close_off];
                let content = trim_code_span(raw);
                out.push(Inline::Code(content));
                i += close_off + run;
                continue;
            }
            // No matching close: emit one literal backtick and continue. The
            // remaining backticks in the run will be re-examined on the next
            // iteration (potentially as a shorter run with its own close).
            push_text(&mut out, "`");
            i += 1;
            continue;
        }
        if let Some(end) = rest.strip_prefix("$$").and_then(|r| r.find("$$")) {
            out.push(Inline::MathInline {
                source: rest[2..end + 2].to_string(),
            });
            i += end + 4;
        } else if let Some(end) = rest.strip_prefix("{{").and_then(|r| r.find("}}")) {
            let name = &rest[2..end + 2];
            if let Some((ns, n)) = name.split_once('.') {
                out.push(Inline::Var {
                    namespace: ns.to_string(),
                    name: n.to_string(),
                });
            } else if !name.is_empty() {
                out.push(Inline::Var {
                    namespace: "vars".to_string(),
                    name: name.to_string(),
                });
            } else {
                out.push(Inline::Text(rest[..end + 4].to_string()));
            }
            i += end + 4;
        } else if let Some(end) = rest.strip_prefix("[^").and_then(|r| r.find(']')) {
            out.push(Inline::FootnoteRef {
                target: rest[2..end + 2].to_string(),
            });
            i += end + 3;
        } else if let Some(end) = rest.strip_prefix("[@").and_then(|r| r.find(']')) {
            out.push(Inline::CitationRef {
                target: rest[2..end + 2].to_string(),
            });
            i += end + 3;
        } else if let Some(end) = rest.strip_prefix("@[").and_then(|r| r.find(']')) {
            out.push(Inline::Ref {
                target: rest[2..end + 2].to_string(),
            });
            i += end + 3;
        } else if let Some(end) = rest.strip_prefix("@{").and_then(|r| r.find('}')) {
            let raw = &rest[2..end + 2];
            if let Some((kind, target)) = raw.split_once(':') {
                out.push(Inline::Mention {
                    kind: kind.to_string(),
                    target: target.to_string(),
                });
            } else {
                out.push(Inline::Text(rest[..end + 3].to_string()));
            }
            i += end + 3;
        } else if let Some(stripped) = rest.strip_prefix("==") {
            if let Some(end) = stripped.find("==") {
                let after = &stripped[end + 2..];
                let (attrs, consumed) = parse_span_suffix(after);
                out.push(Inline::Mark {
                    children: parse_inlines(&stripped[..end]),
                    attrs,
                });
                i += end + 4 + consumed;
            } else {
                push_text(&mut out, "=");
                i += 1;
            }
        } else if let Some(stripped) = rest.strip_prefix("~~") {
            if let Some(end) = stripped.find("~~") {
                out.push(Inline::Strike(parse_inlines(&stripped[..end])));
                i += end + 4;
            } else {
                push_text(&mut out, "~");
                i += 1;
            }
        } else if let Some(stripped) = rest.strip_prefix('~') {
            if let Some(end) = stripped.find('~') {
                out.push(Inline::Sub(parse_inlines(&stripped[..end])));
                i += end + 2;
            } else {
                push_text(&mut out, "~");
                i += 1;
            }
        } else if let Some(stripped) = rest.strip_prefix('^') {
            if let Some(end) = stripped.find('^') {
                out.push(Inline::Sup(parse_inlines(&stripped[..end])));
                i += end + 2;
            } else {
                push_text(&mut out, "^");
                i += 1;
            }
        } else if let Some(stripped) = rest.strip_prefix("**") {
            if let Some(end) = stripped.find("**") {
                out.push(Inline::Strong(parse_inlines(&stripped[..end])));
                i += end + 4;
            } else {
                push_text(&mut out, "*");
                i += 1;
            }
        } else if let Some(stripped) = rest.strip_prefix('*') {
            if let Some(end) = stripped.find('*') {
                out.push(Inline::Em(parse_inlines(&stripped[..end])));
                i += end + 2;
            } else {
                push_text(&mut out, "*");
                i += 1;
            }
        } else if rest.starts_with("__") || rest.starts_with('_') {
            // Underscore emphasis (RFC §12). CommonMark "intraword underscore"
            // rule: a `_` run can open emphasis only if it is left-flanking
            // and either not right-flanking or preceded by ASCII punctuation;
            // it can close only if it is right-flanking and either not
            // left-flanking or followed by ASCII punctuation. This keeps
            // identifiers like `snake_case`, `__init__`, and `_foo_bar_`
            // partially-literal (CommonMark-compatible).
            let run = if rest.starts_with("__") { 2 } else { 1 };
            let opener_preceding = if i == 0 { None } else { Some(bytes[i - 1]) };
            let opener_following = bytes.get(i + run).copied();
            if can_open_underscore(opener_preceding, opener_following)
                && let Some(close_off) = find_underscore_close(rest, run, bytes, i)
            {
                let inner = &rest[run..run + close_off];
                out.push(if run == 2 {
                    Inline::Strong(parse_inlines(inner))
                } else {
                    Inline::Em(parse_inlines(inner))
                });
                i += run + close_off + run;
                continue;
            }
            push_text(&mut out, "_");
            i += 1;
        } else if rest.starts_with("[[") {
            if let Some(close) = rest.find("]]") {
                let label = &rest[2..close];
                let (attrs, consumed) = parse_span_suffix(&rest[close + 2..]);
                if consumed > 0 {
                    out.push(Inline::Span {
                        children: parse_inlines(label),
                        attrs,
                    });
                    i += close + 2 + consumed;
                    continue;
                }
            }
            push_text(&mut out, "[");
            i += 1;
        } else if rest.starts_with('[') {
            if let Some(close) = rest.find(']') {
                let label = &rest[1..close];
                let after = &rest[close + 1..];
                if let Some(stripped) = after.strip_prefix('(') {
                    if let Some(end) = stripped.find(')') {
                        let after_link = &stripped[end + 1..];
                        let (attrs, consumed_attrs) = if after_link.starts_with('{') {
                            if let Some(attr_end) = after_link.find('}') {
                                (
                                    parse_attrs(&after_link[..=attr_end]).unwrap_or_default(),
                                    attr_end + 1,
                                )
                            } else {
                                (Default::default(), 0)
                            }
                        } else {
                            (Default::default(), 0)
                        };
                        out.push(Inline::Link {
                            label: parse_inlines(label),
                            target: stripped[..end].to_string(),
                            attrs,
                        });
                        i += close + 1 + end + 2 + consumed_attrs;
                        continue;
                    }
                } else if after.starts_with('{') {
                    let (attrs, consumed) = parse_span_suffix(after);
                    if consumed == 0 {
                        push_text(&mut out, "[");
                        i += 1;
                        continue;
                    }
                    out.push(Inline::Span {
                        children: parse_inlines(label),
                        attrs,
                    });
                    i += close + 1 + consumed;
                    continue;
                }
            }
            push_text(&mut out, "[");
            i += 1;
        } else if rest.starts_with('\\') && rest.len() > 1 {
            let next = rest.as_bytes()[1];
            // Hard line break: a backslash immediately before a newline (and
            // there is more content after the newline so we are not at end of
            // input) emits LineBreak and consumes both `\` and `\n`.
            if next == b'\n' {
                out.push(Inline::LineBreak);
                i += 2;
                continue;
            }
            let ch = rest[1..].chars().next().unwrap();
            // Extended escape set: PR2 adds `_ ! . - + < > \ " '`.
            if "`*[](){}#@~^=:|_!.-+<>\\\"'".contains(ch) {
                push_text(&mut out, &ch.to_string());
                i += 1 + ch.len_utf8();
            } else {
                push_text(&mut out, "\\");
                i += 1;
            }
        } else {
            let ch = rest.chars().next().unwrap();
            push_text(&mut out, &ch.to_string());
            i += ch.len_utf8();
        }
    }
    out
}

/// Locate the closing run of exactly `close_len` backticks for a code span
/// opened with `open_len` backticks. The closing run must not be part of a
/// longer run (CommonMark rule). Returns the byte offset *within `rest`* of
/// the first byte of the closing run.
fn find_backtick_run(rest: &str, open_len: usize, close_len: usize) -> Option<usize> {
    let bytes = rest.as_bytes();
    let mut i = open_len;
    while i < bytes.len() {
        if bytes[i] == b'`' {
            let run_start = i;
            while i < bytes.len() && bytes[i] == b'`' {
                i += 1;
            }
            if i - run_start == close_len {
                return Some(run_start);
            }
        } else {
            i += 1;
        }
    }
    None
}

/// CommonMark code-span content normalization: if the content starts and ends
/// with a single space (or newline) AND contains a non-space character, strip
/// a single leading and trailing space. Otherwise the content is preserved
/// verbatim. Internal newlines are normalized to a single space because
/// inline parsing already operates on joined paragraph text.
fn trim_code_span(raw: &str) -> String {
    // Newlines inside a code span become spaces — paragraph join already does
    // most of the work, but a code span on a single line containing literal
    // `\n` (we do not emit such today) would otherwise leak through.
    let normalized: String = raw
        .chars()
        .map(|c| if c == '\n' { ' ' } else { c })
        .collect();
    let bytes = normalized.as_bytes();
    if bytes.len() >= 2
        && bytes[0] == b' '
        && bytes[bytes.len() - 1] == b' '
        && bytes.iter().any(|b| *b != b' ')
    {
        normalized[1..bytes.len() - 1].to_string()
    } else {
        normalized
    }
}

/// Find the first `_` run inside `rest` (after the opening run of `run`
/// underscores) that satisfies the CommonMark "can close underscore" rule.
/// Returns the byte offset *inside `rest[run..]`* of the start of the
/// closing run, or `None`. The opening run starts at `opener_abs_idx`
/// in the full input (used only for diagnostics; the closing flank is
/// fully determined by the bytes around the candidate close).
fn find_underscore_close(
    rest: &str,
    run: usize,
    _input_bytes: &[u8],
    _opener_abs_idx: usize,
) -> Option<usize> {
    let bytes = rest.as_bytes();
    let mut j = run;
    while j < bytes.len() {
        if bytes[j] != b'_' {
            j += 1;
            continue;
        }
        let run_start = j;
        while j < bytes.len() && bytes[j] == b'_' {
            j += 1;
        }
        let run_len = j - run_start;
        if run_len < run {
            continue;
        }
        let preceding = if run_start == 0 {
            None
        } else {
            Some(bytes[run_start - 1])
        };
        let following = bytes.get(j).copied();
        if can_close_underscore(preceding, following) {
            return Some(run_start - run);
        }
    }
    None
}

/// CommonMark left-flank: followed by non-whitespace and either not preceded
/// by non-whitespace, or preceded by ASCII punctuation. "Punctuation" here
/// is any ASCII character that is neither alphanumeric nor whitespace —
/// matches CommonMark's pragmatic definition for ASCII inputs.
fn is_left_flanking(preceding: Option<u8>, following: Option<u8>) -> bool {
    match following {
        Some(b) if !is_ascii_whitespace(b) => match preceding {
            None => true,
            Some(p) if is_ascii_whitespace(p) => true,
            Some(p) if is_ascii_punct(p) => true,
            _ => false,
        },
        _ => false,
    }
}

fn is_right_flanking(preceding: Option<u8>, following: Option<u8>) -> bool {
    match preceding {
        Some(b) if !is_ascii_whitespace(b) => match following {
            None => true,
            Some(f) if is_ascii_whitespace(f) => true,
            Some(f) if is_ascii_punct(f) => true,
            _ => false,
        },
        _ => false,
    }
}

/// Underscore can open emphasis iff it is left-flanking AND
/// (not right-flanking OR preceded by ASCII punctuation).
fn can_open_underscore(preceding: Option<u8>, following: Option<u8>) -> bool {
    if !is_left_flanking(preceding, following) {
        return false;
    }
    if !is_right_flanking(preceding, following) {
        return true;
    }
    matches!(preceding, Some(p) if is_ascii_punct(p))
}

/// Underscore can close emphasis iff it is right-flanking AND
/// (not left-flanking OR followed by ASCII punctuation).
fn can_close_underscore(preceding: Option<u8>, following: Option<u8>) -> bool {
    if !is_right_flanking(preceding, following) {
        return false;
    }
    if !is_left_flanking(preceding, following) {
        return true;
    }
    matches!(following, Some(f) if is_ascii_punct(f))
}

fn is_ascii_punct(b: u8) -> bool {
    // Any ASCII char that is neither alphanumeric nor whitespace counts as
    // punctuation for flanking purposes. Matches CommonMark's ASCII subset.
    b.is_ascii() && !b.is_ascii_alphanumeric() && !is_ascii_whitespace(b)
}

fn is_ascii_whitespace(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r')
}

fn parse_span_suffix(input: &str) -> (Attrs, usize) {
    let mut attrs = Attrs::default();
    let mut consumed = 0;
    let mut saw = false;
    while consumed < input.len() {
        let rest = &input[consumed..];
        if rest.starts_with('{') {
            let Some(end) = rest.find('}') else { break };
            if let Some(parsed) = parse_attrs(&rest[..=end]) {
                attrs.id = parsed.id.or(attrs.id);
                attrs.classes.extend(parsed.classes);
                attrs.attrs.extend(parsed.attrs);
                attrs.styles.extend(parsed.styles);
                consumed += end + 1;
                saw = true;
                continue;
            }
        }
        if rest.starts_with('.') {
            let len = class_suffix_len(rest);
            if len == 0 {
                break;
            }
            merge_class_suffix(&mut attrs, &rest[..len]);
            consumed += len;
            saw = true;
            continue;
        }
        break;
    }
    attrs.classes.sort();
    attrs.classes.dedup();
    (attrs, if saw { consumed } else { 0 })
}

fn class_suffix_len(input: &str) -> usize {
    let mut pos = 0;
    while input[pos..].starts_with('.') {
        let rest = &input[pos + 1..];
        let mut chars = rest.chars();
        let Some(first) = chars.next() else { break };
        if !(first.is_ascii_alphabetic() || first == '_') {
            break;
        }
        pos += 1 + first.len_utf8();
        for ch in chars {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                pos += ch.len_utf8();
            } else {
                break;
            }
        }
    }
    pos
}

fn push_text(out: &mut Vec<Inline>, text: &str) {
    if let Some(Inline::Text(prev)) = out.last_mut() {
        prev.push_str(text);
    } else {
        out.push(Inline::Text(text.to_string()));
    }
}

pub fn plain_node_text(node: &Node) -> String {
    let mut out = plain_inlines(&node.inlines);
    for child in &node.children {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(&plain_node_text(child));
    }
    out
}

pub fn plain_inlines(inlines: &[Inline]) -> String {
    let mut out = String::new();
    for item in inlines {
        match item {
            Inline::Text(s) | Inline::Code(s) | Inline::MathInline { source: s } => out.push_str(s),
            Inline::Strong(children)
            | Inline::Em(children)
            | Inline::Strike(children)
            | Inline::Sub(children)
            | Inline::Sup(children) => out.push_str(&plain_inlines(children)),
            Inline::Mark { children, .. } => out.push_str(&plain_inlines(children)),
            Inline::Link { label, .. }
            | Inline::Span {
                children: label, ..
            } => {
                out.push_str(&plain_inlines(label));
            }
            Inline::Var { namespace, name } => {
                out.push_str("{{");
                out.push_str(namespace);
                out.push('.');
                out.push_str(name);
                out.push_str("}}");
            }
            Inline::Ref { target }
            | Inline::FootnoteRef { target }
            | Inline::CitationRef { target } => {
                out.push_str(target);
            }
            Inline::Mention { kind, target } => {
                out.push('@');
                out.push_str(kind);
                out.push(':');
                out.push_str(target);
            }
            Inline::LineBreak => out.push(' '),
        }
    }
    out
}

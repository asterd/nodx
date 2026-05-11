use crate::ast::{Inline, Node};
use crate::attrs::parse_attrs;

pub fn parse_inlines(input: &str) -> Vec<Inline> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < input.len() {
        let rest = &input[i..];
        if let Some(end) = rest.strip_prefix('`').and_then(|r| r.find('`')) {
            out.push(Inline::Code(rest[1..end + 1].to_string()));
            i += end + 2;
        } else if let Some(end) = rest.strip_prefix("$$").and_then(|r| r.find("$$")) {
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
                out.push(Inline::Mark(parse_inlines(&stripped[..end])));
                i += end + 4;
            } else {
                push_text(&mut out, "=");
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
                } else if after.starts_with('{')
                    && let Some(end) = after.find('}')
                    && let Some(attrs) = parse_attrs(&after[..=end])
                {
                    out.push(Inline::Span {
                        children: parse_inlines(label),
                        attrs,
                    });
                    i += close + 1 + end + 1;
                    continue;
                }
            }
            push_text(&mut out, "[");
            i += 1;
        } else if rest.starts_with('\\') && rest.len() > 1 {
            let ch = rest[1..].chars().next().unwrap();
            if "`*[](){}#@~^=:|".contains(ch) {
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
            | Inline::Mark(children)
            | Inline::Sub(children)
            | Inline::Sup(children) => out.push_str(&plain_inlines(children)),
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
        }
    }
    out
}

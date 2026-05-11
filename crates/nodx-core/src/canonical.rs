use std::collections::BTreeMap;

use crate::ast::{Attrs, Document, Inline, Node, Value};

pub fn canonical_json(doc: &Document) -> String {
    let mut s = String::new();
    write_document(&mut s, doc);
    s
}

fn write_document(out: &mut String, doc: &Document) {
    out.push_str("{\"body\":");
    write_nodes(out, &doc.body);
    out.push_str(",\"meta\":");
    write_map_value(out, &doc.meta);
    out.push_str(",\"schema\":\"");
    escape_json(out, &doc.schema);
    out.push_str("\"}");
}

fn write_nodes(out: &mut String, nodes: &[Node]) {
    out.push('[');
    for (i, node) in nodes.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        write_node(out, node);
    }
    out.push(']');
}

fn write_node(out: &mut String, node: &Node) {
    out.push_str("{\"attrs\":");
    write_str_map(out, &node.attrs);
    out.push_str(",\"children\":");
    write_nodes(out, &node.children);
    out.push_str(",\"classes\":");
    write_str_list(out, &node.classes);
    out.push_str(",\"id\":");
    match &node.id {
        Some(id) => write_json_string(out, id),
        None => out.push_str("null"),
    }
    out.push_str(",\"inlines\":");
    write_inlines(out, &node.inlines);
    out.push_str(",\"text\":");
    match &node.text {
        Some(text) => write_json_string(out, text),
        None => out.push_str("null"),
    }
    out.push_str(",\"type\":");
    write_json_string(out, &node.node_type);
    out.push('}');
}

fn write_inlines(out: &mut String, inlines: &[Inline]) {
    out.push('[');
    for (i, item) in inlines.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        match item {
            Inline::Text(text) => {
                out.push_str("{\"text\":");
                write_json_string(out, text);
                out.push_str(",\"type\":\"text\"}");
            }
            Inline::Strong(children) => inline_children(out, "strong", children),
            Inline::Em(children) => inline_children(out, "em", children),
            Inline::Mark(children) => inline_children(out, "mark", children),
            Inline::Sub(children) => inline_children(out, "sub", children),
            Inline::Sup(children) => inline_children(out, "sup", children),
            Inline::Code(text) => {
                out.push_str("{\"text\":");
                write_json_string(out, text);
                out.push_str(",\"type\":\"code\"}");
            }
            Inline::Link {
                label,
                target,
                attrs,
            } => {
                out.push('{');
                if attrs != &Attrs::default() {
                    out.push_str("\"attrs\":");
                    write_attrs(out, attrs);
                    out.push(',');
                }
                out.push_str("\"label\":");
                write_inlines(out, label);
                out.push_str(",\"target\":");
                write_json_string(out, target);
                out.push_str(",\"type\":\"link\"}");
            }
            Inline::Span { children, attrs } => {
                out.push_str("{\"attrs\":");
                write_attrs(out, attrs);
                out.push_str(",\"children\":");
                write_inlines(out, children);
                out.push_str(",\"type\":\"span\"}");
            }
            Inline::Var { namespace, name } => {
                out.push_str("{\"name\":");
                write_json_string(out, name);
                out.push_str(",\"namespace\":");
                write_json_string(out, namespace);
                out.push_str(",\"type\":\"var\"}");
            }
            Inline::Ref { target } => {
                out.push_str("{\"target\":");
                write_json_string(out, target);
                out.push_str(",\"type\":\"ref\"}");
            }
            Inline::Mention { kind, target } => {
                out.push_str("{\"kind\":");
                write_json_string(out, kind);
                out.push_str(",\"target\":");
                write_json_string(out, target);
                out.push_str(",\"type\":\"mention\"}");
            }
            Inline::FootnoteRef { target } => {
                out.push_str("{\"target\":");
                write_json_string(out, target);
                out.push_str(",\"type\":\"footnote-ref\"}");
            }
            Inline::CitationRef { target } => {
                out.push_str("{\"target\":");
                write_json_string(out, target);
                out.push_str(",\"type\":\"citation-ref\"}");
            }
            Inline::MathInline { source } => {
                out.push_str("{\"source\":");
                write_json_string(out, source);
                out.push_str(",\"type\":\"math-inline\"}");
            }
        }
    }
    out.push(']');
}

fn inline_children(out: &mut String, typ: &str, children: &[Inline]) {
    out.push_str("{\"children\":");
    write_inlines(out, children);
    out.push_str(",\"type\":");
    write_json_string(out, typ);
    out.push('}');
}

fn write_attrs(out: &mut String, attrs: &Attrs) {
    out.push_str("{\"attrs\":");
    write_str_map(out, &attrs.attrs);
    out.push_str(",\"classes\":");
    write_str_list(out, &attrs.classes);
    out.push_str(",\"id\":");
    match &attrs.id {
        Some(id) => write_json_string(out, id),
        None => out.push_str("null"),
    }
    out.push('}');
}

fn write_map_value(out: &mut String, map: &BTreeMap<String, Value>) {
    out.push('{');
    for (i, (k, v)) in map.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        write_json_string(out, k);
        out.push(':');
        write_value(out, v);
    }
    out.push('}');
}

fn write_value(out: &mut String, value: &Value) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(v) => out.push_str(if *v { "true" } else { "false" }),
        Value::Number(v) => out.push_str(v),
        Value::String(v) => write_json_string(out, v),
        Value::List(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_value(out, item);
            }
            out.push(']');
        }
        Value::Map(map) => write_map_value(out, map),
    }
}

pub fn write_str_map(out: &mut String, map: &BTreeMap<String, String>) {
    out.push('{');
    for (i, (k, v)) in map.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        write_json_string(out, k);
        out.push(':');
        write_json_string(out, v);
    }
    out.push('}');
}

fn write_str_list(out: &mut String, items: &[String]) {
    out.push('[');
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        write_json_string(out, item);
    }
    out.push(']');
}

pub fn write_json_string(out: &mut String, input: &str) {
    out.push('"');
    escape_json(out, input);
    out.push('"');
}

fn escape_json(out: &mut String, input: &str) {
    for ch in input.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c < ' ' => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
}

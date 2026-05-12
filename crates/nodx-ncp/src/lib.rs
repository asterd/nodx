#![forbid(unsafe_code)]

use nodx_core::{
    Document, NavigationGraph, Node, canonical::write_json_string, canonical::write_str_map,
    canonical_json, plain_inlines, resolve_navigation, sha256_base64url,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NcpMode {
    Semantic,
}

pub fn ncp_json(doc: &Document) -> String {
    ncp_json_with_mode(doc, NcpMode::Semantic)
}

pub fn ncp_json_with_mode(doc: &Document, mode: NcpMode) -> String {
    match mode {
        NcpMode::Semantic => render_semantic(doc),
    }
}

pub fn node_hash(node: &Node) -> String {
    sha256_base64url(node_hash_input(node).as_bytes())
}

pub fn document_hash(doc: &Document) -> String {
    sha256_base64url(canonical_json(doc).as_bytes())
}

pub fn semantic_text(doc: &Document) -> String {
    let mut lines = Vec::new();
    write_semantic_nodes(&doc.body, &mut lines);
    let text = collapse_blank_lines(&lines.join("\n"));
    if text.trim().is_empty() {
        "\n".to_string()
    } else {
        format!("{}\n", text.trim())
    }
}

pub fn node_hash_input(node: &Node) -> String {
    let mut out = String::new();
    out.push_str(&node.node_type);
    out.push('\n');
    if let Some(id) = &node.id {
        out.push_str(id);
    }
    out.push('\n');
    write_str_map(&mut out, &node.attrs);
    out.push('\n');
    if !node.styles.is_empty() {
        write_str_map(&mut out, &node.styles);
        out.push('\n');
    }
    out.push_str(node.text.as_deref().unwrap_or(""));
    out.push_str(&plain_inlines(&node.inlines));
    for child in &node.children {
        out.push('\n');
        out.push_str(&node_hash_input(child));
    }
    out
}

fn write_semantic_nodes(nodes: &[Node], lines: &mut Vec<String>) {
    for node in nodes {
        if is_semantic_text_excluded(&node.node_type) {
            continue;
        }
        write_semantic_node(node, lines);
    }
}

fn write_semantic_node(node: &Node, lines: &mut Vec<String>) {
    match node.node_type.as_str() {
        "heading" => {
            let level = node
                .attrs
                .get("level")
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(1)
                .clamp(1, 6);
            lines.push(format!(
                "{} {}{}",
                "#".repeat(level),
                plain_inlines(&node.inlines),
                semantic_id(node)
            ));
        }
        "paragraph" => push_text_line(lines, &plain_inlines(&node.inlines)),
        "list" => {
            let ordered = matches!(node.attrs.get("kind"), Some(kind) if kind == "ordered");
            let mut index = 1;
            for item in node
                .children
                .iter()
                .filter(|child| child.node_type == "item")
            {
                let marker = if ordered {
                    let marker = format!("{index}.");
                    index += 1;
                    marker
                } else if matches!(item.attrs.get("checked"), Some(value) if value == "true") {
                    "- [x]".to_string()
                } else if matches!(item.attrs.get("checked"), Some(value) if value == "false") {
                    "- [ ]".to_string()
                } else {
                    "-".to_string()
                };
                lines.push(
                    format!("{marker} {}", semantic_node_text(item))
                        .trim()
                        .to_string(),
                );
            }
        }
        "table" => write_semantic_table(node, lines),
        "figure" => {
            lines.push(format!("Figure{}:", semantic_id(node)));
            write_semantic_nodes(&node.children, lines);
        }
        "image" => lines.push(format!(
            "Image{}: alt=\"{}\" src=\"{}\"",
            semantic_id(node),
            node.attrs.get("alt").map(String::as_str).unwrap_or(""),
            node.attrs.get("src").map(String::as_str).unwrap_or("")
        )),
        "caption" => push_text_line(lines, &format!("Caption: {}", semantic_node_text(node))),
        "code" | "pre" | "math" => {
            let lang = node
                .attrs
                .get("lang")
                .map(|value| format!(" {value}"))
                .unwrap_or_default();
            lines.push(format!("```{}{lang}", node.node_type));
            lines.push(node.text.clone().unwrap_or_default());
            lines.push("```".to_string());
        }
        "quote" => {
            for line in semantic_node_text(node).lines() {
                lines.push(format!("> {line}"));
            }
        }
        "note" => lines.push(
            format!("Note{}: {}", semantic_attrs(node), semantic_node_text(node))
                .trim()
                .to_string(),
        ),
        "form" => {
            lines.push(format!("Form{}:", semantic_id(node)));
            for field in node
                .children
                .iter()
                .filter(|child| child.node_type == "field")
            {
                let label = field
                    .attrs
                    .get("label")
                    .or_else(|| field.attrs.get("name"))
                    .map(String::as_str)
                    .unwrap_or("Field");
                let value = field
                    .attrs
                    .get("value")
                    .cloned()
                    .unwrap_or_else(|| semantic_node_text(field));
                lines.push(format!("- {label}: {value}"));
            }
        }
        "field" => {
            let value = node
                .attrs
                .get("value")
                .cloned()
                .unwrap_or_else(|| semantic_node_text(node));
            lines.push(format!("{}: {value}", field_label(node)));
        }
        "media" | "embed" | "include" => lines.push(
            format!(
                "{}{}: {}",
                capitalize(&node.node_type),
                semantic_attrs(node),
                semantic_node_text(node)
            )
            .trim()
            .to_string(),
        ),
        "bibliography" => {
            lines.push("Bibliography:".to_string());
            write_semantic_nodes(&node.children, lines);
        }
        "citation-entry" => lines.push(format!("- {}", semantic_node_text(node))),
        node_type if node_type.contains('-') => {
            lines.push(format!(
                "Component {}{}:",
                node.node_type,
                semantic_attrs(node)
            ));
            write_semantic_nodes(&node.children, lines);
        }
        _ => {
            let text = semantic_node_text(node);
            if text.is_empty() {
                write_semantic_nodes(&node.children, lines);
            } else {
                lines.push(format!(
                    "{}{}: {}",
                    capitalize(&node.node_type),
                    semantic_attrs(node),
                    text
                ));
            }
        }
    }
}

fn write_semantic_table(node: &Node, lines: &mut Vec<String>) {
    let rows: Vec<Vec<&Node>> = node
        .children
        .iter()
        .filter(|child| child.node_type == "row")
        .map(|row| {
            row.children
                .iter()
                .filter(|cell| cell.node_type == "cell")
                .collect()
        })
        .collect();
    if rows.is_empty() {
        lines.push(format!("Table{}: empty", semantic_id(node)));
        return;
    }
    lines.push(format!("Table{}:", semantic_id(node)));
    let values: Vec<Vec<String>> = rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|cell| escape_markdown_cell(&semantic_node_text(cell)))
                .collect()
        })
        .collect();
    let width = values.iter().map(Vec::len).max().unwrap_or(0);
    let mut normalized = values;
    for row in &mut normalized {
        row.resize(width, String::new());
    }
    let first_row_is_header = rows[0]
        .iter()
        .all(|cell| matches!(cell.attrs.get("header"), Some(value) if value == "true"));
    let header = if first_row_is_header {
        normalized[0].clone()
    } else {
        (1..=width).map(|index| format!("Column {index}")).collect()
    };
    lines.push(format!("| {} |", header.join(" | ")));
    lines.push(format!("| {} |", vec!["---"; width].join(" | ")));
    for row in normalized
        .iter()
        .skip(if first_row_is_header { 1 } else { 0 })
    {
        lines.push(format!("| {} |", row.join(" | ")));
    }
}

fn semantic_node_text(node: &Node) -> String {
    let mut parts = Vec::new();
    let inline = plain_inlines(&node.inlines).trim().to_string();
    if !inline.is_empty() {
        parts.push(inline);
    }
    if let Some(text) = &node.text {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            parts.push(trimmed.to_string());
        }
    }
    for child in &node.children {
        if is_semantic_text_excluded(&child.node_type) {
            continue;
        }
        if matches!(
            child.node_type.as_str(),
            "row" | "cell" | "paragraph" | "caption" | "item"
        ) {
            let text = semantic_node_text(child);
            if !text.is_empty() {
                parts.push(text);
            }
        }
    }
    parts
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_semantic_text_excluded(node_type: &str) -> bool {
    matches!(node_type, "style" | "pagebreak" | "toc")
}

fn semantic_id(node: &Node) -> String {
    node.id
        .as_ref()
        .map(|id| format!(" #{id}"))
        .unwrap_or_default()
}

fn semantic_attrs(node: &Node) -> String {
    let mut attrs = Vec::new();
    if let Some(id) = &node.id {
        attrs.push(format!("id=\"{id}\""));
    }
    for (key, value) in &node.attrs {
        if !value.is_empty() && !matches!(key.as_str(), "level" | "header" | "scope") {
            attrs.push(format!("{key}=\"{value}\""));
        }
    }
    if attrs.is_empty() {
        String::new()
    } else {
        format!(" [{}]", attrs.join(" "))
    }
}

fn field_label(node: &Node) -> &str {
    node.attrs
        .get("label")
        .or_else(|| node.attrs.get("name"))
        .map(String::as_str)
        .unwrap_or("Field")
}

fn push_text_line(lines: &mut Vec<String>, text: &str) {
    let normalized = text.trim();
    if !normalized.is_empty() {
        lines.push(normalized.to_string());
    }
}

fn escape_markdown_cell(text: &str) -> String {
    text.replace('|', "\\|")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
        None => String::new(),
    }
}

fn collapse_blank_lines(text: &str) -> String {
    let mut out = String::new();
    let mut blank_count = 0;
    for line in text.lines() {
        if line.trim().is_empty() {
            blank_count += 1;
            if blank_count <= 2 {
                out.push('\n');
            }
        } else {
            blank_count = 0;
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(line);
        }
    }
    out
}

fn render_semantic(doc: &Document) -> String {
    let canonical = canonical_json(doc);
    let ids = collect_node_ids(&doc.body);
    let chunk_hash = sha256_base64url(ids.join("\n").as_bytes());
    let navigation = resolve_navigation(doc);
    let mut out = String::from("{\"chunks\":[{\"id\":\"chunk-1\",\"nodes\":[");
    for (i, id) in ids.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        write_json_string(&mut out, id);
    }
    out.push_str("],\"sha256\":");
    write_json_string(&mut out, &chunk_hash);
    out.push_str("}],\"loss\":[],\"mode\":\"semantic\",\"nodes\":");
    write_ncp_nodes(&mut out, &doc.body, "", &navigation);
    out.push_str(",\"schema\":\"nodx-ncp/1.0\",\"sourceHash\":");
    write_json_string(&mut out, &sha256_base64url(canonical.as_bytes()));
    out.push('}');
    out
}

fn collect_node_ids(nodes: &[Node]) -> Vec<String> {
    let mut out = Vec::new();
    collect_node_ids_at(nodes, "", &mut out);
    out
}

fn collect_node_ids_at(nodes: &[Node], prefix: &str, out: &mut Vec<String>) {
    for (i, node) in nodes.iter().enumerate() {
        let path = if prefix.is_empty() {
            i.to_string()
        } else {
            format!("{}.{}", prefix, i)
        };
        out.push(node.id.clone().unwrap_or_else(|| format!("path:{path}")));
        collect_node_ids_at(&node.children, &path, out);
    }
}

fn write_ncp_nodes(out: &mut String, nodes: &[Node], prefix: &str, navigation: &NavigationGraph) {
    out.push('[');
    for (i, node) in nodes.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        let path = if prefix.is_empty() {
            i.to_string()
        } else {
            format!("{}.{}", prefix, i)
        };
        write_ncp_node(out, node, &path, navigation);
    }
    out.push(']');
}

fn write_ncp_node(out: &mut String, node: &Node, path: &str, navigation: &NavigationGraph) {
    out.push_str("{\"attrs\":");
    write_str_map(out, &node.attrs);
    out.push_str(",\"children\":");
    write_ncp_nodes(out, &node.children, path, navigation);
    out.push_str(",\"id\":");
    write_json_string(out, node.id.as_deref().unwrap_or(""));
    out.push_str(",\"path\":");
    write_json_string(out, path);
    out.push_str(",\"sha256\":");
    write_json_string(out, &node_hash(node));
    out.push_str(",\"text\":");
    let text = node
        .text
        .clone()
        .unwrap_or_else(|| plain_inlines(&node.inlines));
    write_json_string(out, &text);
    out.push_str(",\"type\":");
    write_json_string(out, &node.node_type);
    if node.node_type == "toc" {
        out.push_str(",\"navigationEntries\":");
        write_navigation_entries(out, path, navigation);
    }
    out.push('}');
}

fn write_navigation_entries(out: &mut String, path: &str, navigation: &NavigationGraph) {
    out.push('[');
    if let Some(nav) = navigation
        .navigations
        .iter()
        .find(|candidate| candidate.toc_path == path)
    {
        for (i, entry) in nav.entries.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str("{\"id\":");
            write_json_string(out, &entry.id);
            out.push_str(",\"level\":");
            out.push_str(&entry.level.to_string());
            out.push_str(",\"path\":");
            write_json_string(out, &entry.path);
            out.push_str(",\"title\":");
            write_json_string(out, &entry.title);
            out.push('}');
        }
    }
    out.push(']');
}

#[cfg(test)]
mod tests {
    use super::*;
    use nodx_core::parse_str;

    #[test]
    fn semantic_projection_is_stable() {
        let doc = parse_str("---\nschema: nodx/1.0\n---\n# Hello {#h}\n");
        let json = ncp_json(&doc);
        assert!(json.contains("\"schema\":\"nodx-ncp/1.0\""));
        assert!(json.contains("\"id\":\"h\""));
    }

    #[test]
    fn node_hash_is_deterministic() {
        let doc = parse_str("---\nschema: nodx/1.0\n---\n# A {#a}\n");
        let a = node_hash(&doc.body[0]);
        let b = node_hash(&doc.body[0]);
        assert_eq!(a, b);
    }
}

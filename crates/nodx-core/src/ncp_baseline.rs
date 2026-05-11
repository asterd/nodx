use crate::ast::{Document, Node};
use crate::bytes::sha256_base64url;
use crate::canonical::{canonical_json, write_json_string, write_str_map};
use crate::inline_parser::plain_inlines;
use crate::navigation::{NavigationGraph, resolve_navigation};

pub fn ncp_json(doc: &Document) -> String {
    let canonical = canonical_json(doc);
    let ids = collect_node_ids(&doc.body);
    let chunk_hash = sha256_base64url(ids.join("\n").as_bytes());
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
    let navigation = resolve_navigation(doc);
    write_ncp_nodes(&mut out, &doc.body, "", &navigation);
    out.push_str(",\"schema\":\"nodx-ncp/0.1\",\"sourceHash\":");
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
    write_json_string(out, &sha256_base64url(ncp_node_hash_input(node).as_bytes()));
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

fn ncp_node_hash_input(node: &Node) -> String {
    let mut out = String::new();
    out.push_str(&node.node_type);
    out.push('\n');
    if let Some(id) = &node.id {
        out.push_str(id);
    }
    out.push('\n');
    write_str_map(&mut out, &node.attrs);
    out.push('\n');
    out.push_str(node.text.as_deref().unwrap_or(""));
    out.push_str(&plain_inlines(&node.inlines));
    for child in &node.children {
        out.push('\n');
        out.push_str(&ncp_node_hash_input(child));
    }
    out
}

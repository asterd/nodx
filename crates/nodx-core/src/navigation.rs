use std::collections::BTreeMap;

use crate::ast::{Document, Node};
use crate::inline_parser::plain_node_text;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NavigationGraph {
    pub navigations: Vec<ResolvedNavigation>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedNavigation {
    pub toc_path: String,
    pub toc_id: Option<String>,
    pub role: String,
    pub label: String,
    pub scope: Option<String>,
    pub entries: Vec<NavigationEntry>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NavigationEntry {
    pub id: String,
    pub level: usize,
    pub title: String,
    pub path: String,
}

pub fn resolve_navigation(doc: &Document) -> NavigationGraph {
    let mut ids = BTreeMap::new();
    collect_id_paths(&doc.body, "", &mut ids);

    let mut navigations = Vec::new();
    collect_tocs(&doc.body, "", doc, &ids, &mut navigations);
    NavigationGraph { navigations }
}

pub fn default_navigation_label(role: &str) -> &'static str {
    match role {
        "local" => "In this section",
        "secondary" => "Secondary navigation",
        "breadcrumb" => "Breadcrumb",
        _ => "Table of contents",
    }
}

fn collect_id_paths(nodes: &[Node], prefix: &str, ids: &mut BTreeMap<String, String>) {
    for (i, node) in nodes.iter().enumerate() {
        let path = child_path(prefix, i);
        if let Some(id) = &node.id {
            ids.insert(id.clone(), path.clone());
        }
        collect_id_paths(&node.children, &path, ids);
    }
}

fn collect_tocs(
    nodes: &[Node],
    prefix: &str,
    doc: &Document,
    ids: &BTreeMap<String, String>,
    out: &mut Vec<ResolvedNavigation>,
) {
    for (i, node) in nodes.iter().enumerate() {
        let path = child_path(prefix, i);
        if node.node_type == "toc" {
            out.push(resolve_toc(node, &path, doc, ids));
        }
        collect_tocs(&node.children, &path, doc, ids, out);
    }
}

fn resolve_toc(
    toc: &Node,
    path: &str,
    doc: &Document,
    ids: &BTreeMap<String, String>,
) -> ResolvedNavigation {
    let role = toc
        .attrs
        .get("role")
        .cloned()
        .unwrap_or_else(|| "primary".to_string());
    let label = toc
        .attrs
        .get("title")
        .cloned()
        .unwrap_or_else(|| default_navigation_label(&role).to_string());
    let scope = toc.attrs.get("scope").cloned();
    let source_nodes = scope
        .as_deref()
        .and_then(|raw| raw.strip_prefix('#'))
        .and_then(|id| ids.get(id))
        .and_then(|scope_path| node_at_path(&doc.body, scope_path))
        .map(|node| std::slice::from_ref(node))
        .unwrap_or(&doc.body);

    let min_level = toc.attrs.get("min-level").and_then(|v| parse_level(v));
    let max_level = toc.attrs.get("max-level").and_then(|v| parse_level(v));
    let depth = toc
        .attrs
        .get("depth")
        .and_then(|v| parse_level(v))
        .unwrap_or(6);
    let base_level = first_heading_level(source_nodes).unwrap_or(1);
    let effective_min = min_level.unwrap_or(base_level);
    let effective_max = max_level.unwrap_or((base_level + depth - 1).min(6));

    let mut entries = Vec::new();
    collect_entries(source_nodes, "", effective_min, effective_max, &mut entries);
    ResolvedNavigation {
        toc_path: path.to_string(),
        toc_id: toc.id.clone(),
        role,
        label,
        scope,
        entries,
    }
}

fn collect_entries(
    nodes: &[Node],
    prefix: &str,
    min_level: usize,
    max_level: usize,
    out: &mut Vec<NavigationEntry>,
) {
    for (i, node) in nodes.iter().enumerate() {
        let path = child_path(prefix, i);
        if node.node_type == "heading" {
            if let (Some(id), Some(level)) = (&node.id, heading_level(node)) {
                if (min_level..=max_level).contains(&level) {
                    out.push(NavigationEntry {
                        id: id.clone(),
                        level,
                        title: plain_node_text(node),
                        path: path.clone(),
                    });
                }
            }
        }
        collect_entries(&node.children, &path, min_level, max_level, out);
    }
}

fn first_heading_level(nodes: &[Node]) -> Option<usize> {
    for node in nodes {
        if node.node_type == "heading" {
            if let Some(level) = heading_level(node) {
                return Some(level);
            }
        }
        if let Some(level) = first_heading_level(&node.children) {
            return Some(level);
        }
    }
    None
}

fn node_at_path<'a>(nodes: &'a [Node], path: &str) -> Option<&'a Node> {
    let mut current = nodes;
    let mut node = None;
    for part in path.split('.') {
        let index = part.parse::<usize>().ok()?;
        let next = current.get(index)?;
        node = Some(next);
        current = &next.children;
    }
    node
}

fn heading_level(node: &Node) -> Option<usize> {
    node.attrs.get("level").and_then(|v| parse_level(v))
}

fn parse_level(value: &str) -> Option<usize> {
    value
        .parse::<usize>()
        .ok()
        .filter(|level| (1..=6).contains(level))
}

fn child_path(prefix: &str, index: usize) -> String {
    if prefix.is_empty() {
        index.to_string()
    } else {
        format!("{prefix}.{index}")
    }
}

#[cfg(test)]
mod tests {
    use crate::{parse_str, resolve_navigation};

    #[test]
    fn resolves_document_toc_in_order() {
        let doc = parse_str(
            ":::toc {#nav depth=\"2\"}\n:::\n\n# One {#one}\n\n## Two {#two}\n\n### Three {#three}\n",
        );
        let graph = resolve_navigation(&doc);
        assert_eq!(graph.navigations.len(), 1);
        let nav = &graph.navigations[0];
        assert_eq!(nav.label, "Table of contents");
        let ids: Vec<_> = nav.entries.iter().map(|entry| entry.id.as_str()).collect();
        assert_eq!(ids, vec!["one", "two"]);
    }

    #[test]
    fn resolves_scoped_toc() {
        let doc = parse_str(
            ":::section {#a}\n# A {#ha}\n:::\n\n:::section {#b}\n:::toc {scope=\"#b\" role=\"local\"}\n:::\n# B {#hb}\n## B2 {#hb2}\n:::\n",
        );
        let graph = resolve_navigation(&doc);
        let nav = &graph.navigations[0];
        assert_eq!(nav.label, "In this section");
        let ids: Vec<_> = nav.entries.iter().map(|entry| entry.id.as_str()).collect();
        assert_eq!(ids, vec!["hb", "hb2"]);
    }
}

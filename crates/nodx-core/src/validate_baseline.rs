use std::collections::BTreeSet;

use crate::ast::{Document, Inline, Node, Value};
use crate::attrs::valid_name;
use crate::diagnostic::Diagnostic;
use crate::html_baseline::{is_safe_asset_ref, safe_link_url};
use crate::limits::ResourceLimits;

pub(crate) fn validate_document(doc: &mut Document) {
    validate_meta(doc);
    let declared_components = component_names(doc);
    let declared_vars = declared_vars(doc);
    let mut ids = BTreeSet::new();
    let mut refs = Vec::new();
    let mut previous_heading = 0usize;
    validate_nodes(
        &doc.body,
        &mut ids,
        &mut refs,
        &declared_components,
        &declared_vars,
        &mut previous_heading,
        &mut doc.diagnostics,
    );
    for target in refs {
        if !ids.contains(&target) {
            doc.diagnostics.push(Diagnostic {
                code: "NODX-E007".to_string(),
                severity: "error".to_string(),
                message: format!("Unresolved reference `#{}`.", target),
                line: None,
                column: None,
                target: Some(format!("#{target}")),
            });
        }
    }
}

fn validate_meta(doc: &mut Document) {
    match doc.meta.get("schema") {
        Some(Value::String(schema)) if schema == "nodx/0.1" => {}
        _ => doc.diagnostics.push(Diagnostic {
            code: "NODX-E004".to_string(),
            severity: "error".to_string(),
            message: "Missing or invalid schema for NODX 0.1.".to_string(),
            line: None,
            column: None,
            target: None,
        }),
    }
    if let Some(Value::List(required)) = doc.meta.get("requires") {
        for item in required {
            if let Value::String(feature) = item {
                if !supported_feature(feature) {
                    doc.diagnostics.push(Diagnostic {
                        code: "NODX-E024".to_string(),
                        severity: "error".to_string(),
                        message: format!("Required feature `{}` is unsupported.", feature),
                        line: None,
                        column: None,
                        target: None,
                    });
                }
            }
        }
    }
}

fn supported_feature(feature: &str) -> bool {
    matches!(
        feature,
        "rich-tables" | "math" | "media" | "custom-components" | "style"
    )
}

fn component_names(doc: &Document) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    if let Some(Value::List(items)) = doc.meta.get("components") {
        for item in items {
            if let Value::Map(map) = item {
                if let Some(Value::String(name)) = map.get("name") {
                    out.insert(name.clone());
                }
            }
        }
    }
    out
}

fn declared_vars(doc: &Document) -> BTreeSet<String> {
    match doc.meta.get("vars") {
        Some(Value::Map(vars)) => vars.keys().cloned().collect(),
        _ => BTreeSet::new(),
    }
}

fn validate_nodes(
    nodes: &[Node],
    ids: &mut BTreeSet<String>,
    refs: &mut Vec<String>,
    components: &BTreeSet<String>,
    vars: &BTreeSet<String>,
    previous_heading: &mut usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let limits = ResourceLimits::default();
    for node in nodes {
        if let Some(id) = &node.id {
            if !valid_name(id, true) || id.len() > limits.id_bytes {
                diagnostics.push(validation_diag(
                    "NODX-E004",
                    "error",
                    "Invalid node id.",
                    id,
                ));
            }
            if !ids.insert(id.clone()) {
                diagnostics.push(validation_diag(
                    "NODX-E006",
                    "error",
                    "Duplicate node id.",
                    id,
                ));
            }
        }
        validate_common_attrs(node, diagnostics);
        if node.node_type.contains('-')
            && !is_standard_node(&node.node_type)
            && !components.contains(&node.node_type)
            && !node.attrs.contains_key("fallback")
        {
            diagnostics.push(validation_diag(
                "NODX-E014",
                "warning",
                "Custom component is not declared and has no explicit fallback.",
                &node.node_type,
            ));
        }
        match node.node_type.as_str() {
            "heading" => validate_heading(node, previous_heading, diagnostics),
            "image" => validate_image(node, diagnostics),
            "media" | "embed" | "include" => validate_asset_node(node, diagnostics),
            "table" => validate_table(node, diagnostics),
            _ => {}
        }
        collect_inline_refs(&node.inlines, refs, vars, diagnostics);
        validate_nodes(
            &node.children,
            ids,
            refs,
            components,
            vars,
            previous_heading,
            diagnostics,
        );
    }
}

fn validate_common_attrs(node: &Node, diagnostics: &mut Vec<Diagnostic>) {
    if let Some(dir) = node.attrs.get("dir") {
        if !matches!(dir.as_str(), "ltr" | "rtl" | "auto") {
            diagnostics.push(validation_diag(
                "NODX-E004",
                "error",
                "Invalid dir attribute.",
                dir,
            ));
        }
    }
}

fn is_standard_node(node_type: &str) -> bool {
    matches!(node_type, "citation-entry" | "pagebreak" | "speaker-notes")
}

fn validate_heading(node: &Node, previous_heading: &mut usize, diagnostics: &mut Vec<Diagnostic>) {
    let level = node
        .attrs
        .get("level")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(1);
    if !(1..=6).contains(&level) {
        diagnostics.push(validation_diag(
            "NODX-E004",
            "error",
            "Heading level must be 1 through 6.",
            &level.to_string(),
        ));
    }
    if *previous_heading > 0 && level > *previous_heading + 1 {
        diagnostics.push(validation_diag(
            "NODX-E022",
            "warning",
            "Heading level jumps over an intermediate level.",
            &level.to_string(),
        ));
    }
    *previous_heading = level;
}

fn validate_image(node: &Node, diagnostics: &mut Vec<Diagnostic>) {
    let decorative = node.attrs.get("decorative").map(String::as_str) == Some("true");
    let alt = node.attrs.get("alt").map(String::as_str).unwrap_or("");
    if !decorative && alt.trim().is_empty() {
        diagnostics.push(validation_diag(
            "NODX-E009",
            "error",
            "Informative image requires non-empty alt text.",
            node.id.as_deref().unwrap_or("image"),
        ));
    }
    validate_asset_node(node, diagnostics);
}

fn validate_asset_node(node: &Node, diagnostics: &mut Vec<Diagnostic>) {
    if let Some(src) = node.attrs.get("src") {
        if !is_safe_asset_ref(src) {
            diagnostics.push(validation_diag(
                "NODX-E010",
                "error",
                "Unsafe asset path.",
                src,
            ));
        }
    }
}

fn validate_table(node: &Node, diagnostics: &mut Vec<Diagnostic>) {
    let mut width = None;
    for row in &node.children {
        if row.node_type != "row" {
            continue;
        }
        let cells = row
            .children
            .iter()
            .filter(|child| child.node_type == "cell")
            .count();
        match width {
            Some(expected) if expected != cells => diagnostics.push(validation_diag(
                "NODX-E025",
                "error",
                "Table rows must have the same number of cells.",
                node.id.as_deref().unwrap_or("table"),
            )),
            None => width = Some(cells),
            _ => {}
        }
    }
}

fn collect_inline_refs(
    inlines: &[Inline],
    refs: &mut Vec<String>,
    vars: &BTreeSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for item in inlines {
        match item {
            Inline::Strong(children)
            | Inline::Em(children)
            | Inline::Mark(children)
            | Inline::Sub(children)
            | Inline::Sup(children) => collect_inline_refs(children, refs, vars, diagnostics),
            Inline::Link { label, target } => {
                if safe_link_url(target).is_none() {
                    diagnostics.push(validation_diag(
                        "NODX-E020",
                        "error",
                        "Unsafe URL or scheme.",
                        target,
                    ));
                }
                collect_inline_refs(label, refs, vars, diagnostics);
            }
            Inline::Span { children, attrs } => {
                if let Some(dir) = &attrs.attrs.get("dir") {
                    if !matches!(dir.as_str(), "ltr" | "rtl" | "auto") {
                        diagnostics.push(validation_diag(
                            "NODX-E004",
                            "error",
                            "Invalid inline dir attribute.",
                            dir,
                        ));
                    }
                }
                collect_inline_refs(children, refs, vars, diagnostics);
            }
            Inline::Var { namespace, name } => {
                if namespace == "vars" && !vars.contains(name) {
                    diagnostics.push(validation_diag(
                        "NODX-E013",
                        "warning",
                        "Variable referenced but not declared.",
                        name,
                    ));
                }
            }
            Inline::Ref { target }
            | Inline::FootnoteRef { target }
            | Inline::CitationRef { target } => {
                refs.push(target.clone());
            }
            Inline::Text(_)
            | Inline::Code(_)
            | Inline::Mention { .. }
            | Inline::MathInline { .. } => {}
        }
    }
}

fn validation_diag(code: &str, severity: &str, message: &str, target: &str) -> Diagnostic {
    Diagnostic {
        code: code.to_string(),
        severity: severity.to_string(),
        message: message.to_string(),
        line: None,
        column: None,
        target: Some(target.to_string()),
    }
}

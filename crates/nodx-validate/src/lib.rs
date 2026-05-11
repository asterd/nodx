#![forbid(unsafe_code)]

use std::collections::BTreeSet;

use nodx_core::{
    Diagnostic, Document, Inline, Node, ResourceLimits, Value, default_navigation_label,
    resolve_navigation, valid_name,
};
use nodx_url::{ReferenceKind, ResourcePolicy};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileSet {
    supported: BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Validator {
    profiles: ProfileSet,
    limits: ResourceLimits,
}

impl Default for ProfileSet {
    fn default() -> Self {
        Self::new(["plain", "core", "rich", "style", "package", "agent-read"])
    }
}

impl ProfileSet {
    pub fn new<I, S>(profiles: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            supported: profiles.into_iter().map(Into::into).collect(),
        }
    }

    pub fn supports(&self, profile: &str) -> bool {
        self.supported.contains(profile) || legacy_supported_feature(profile)
    }
}

impl Default for Validator {
    fn default() -> Self {
        Self {
            profiles: ProfileSet::default(),
            limits: ResourceLimits::default(),
        }
    }
}

impl Validator {
    pub fn new(profiles: ProfileSet) -> Self {
        Self {
            profiles,
            limits: ResourceLimits::default(),
        }
    }

    pub fn with_limits(profiles: ProfileSet, limits: ResourceLimits) -> Self {
        Self { profiles, limits }
    }

    pub fn validate(&self, doc: &Document) -> Vec<Diagnostic> {
        let mut diagnostics = doc.diagnostics.clone();
        self.validate_into(doc, &mut diagnostics, None);
        diagnostics
    }

    pub fn validate_with_profile(&self, doc: &Document, profile: Option<&str>) -> Vec<Diagnostic> {
        let mut diagnostics = doc.diagnostics.clone();
        self.validate_into(doc, &mut diagnostics, profile);
        diagnostics
    }

    fn validate_into(
        &self,
        doc: &Document,
        diagnostics: &mut Vec<Diagnostic>,
        requested_profile: Option<&str>,
    ) {
        validate_meta(doc, &self.profiles, requested_profile, diagnostics);
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
            diagnostics,
            self.limits,
        );
        validate_navigation(doc, &ids, diagnostics);
        for target in refs {
            if !ids.contains(&target) {
                diagnostics.push(Diagnostic {
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
}

pub fn diagnostics_json(diagnostics: &[Diagnostic]) -> String {
    let mut out = String::from("[");
    for (i, diagnostic) in diagnostics.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str("{\"code\":");
        write_json_string(&mut out, &diagnostic.code);
        out.push_str(",\"severity\":");
        write_json_string(&mut out, &diagnostic.severity);
        out.push_str(",\"message\":");
        write_json_string(&mut out, &diagnostic.message);
        out.push_str(",\"line\":");
        write_json_option_usize(&mut out, diagnostic.line);
        out.push_str(",\"column\":");
        write_json_option_usize(&mut out, diagnostic.column);
        out.push_str(",\"target\":");
        match &diagnostic.target {
            Some(target) => write_json_string(&mut out, target),
            None => out.push_str("null"),
        }
        out.push('}');
    }
    out.push(']');
    out
}

pub fn exit_code_for(diagnostics: &[Diagnostic]) -> i32 {
    if diagnostics
        .iter()
        .any(|d| d.code == "NODX-E024" && (d.severity == "fatal" || d.severity == "error"))
    {
        3
    } else if diagnostics
        .iter()
        .any(|d| d.severity == "fatal" || d.severity == "error")
    {
        2
    } else {
        0
    }
}

fn validate_meta(
    doc: &Document,
    profiles: &ProfileSet,
    requested_profile: Option<&str>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match doc.meta.get("schema") {
        Some(Value::String(schema)) if schema == "nodx/0.1" || schema == "nodx/1.0" => {}
        _ => diagnostics.push(Diagnostic {
            code: "NODX-E004".to_string(),
            severity: "error".to_string(),
            message: "Missing or invalid schema for NODX.".to_string(),
            line: None,
            column: None,
            target: Some("schema".to_string()),
        }),
    }

    if let Some(profile) = requested_profile {
        validate_required_profile(profile, profiles, diagnostics);
    }

    if let Some(Value::List(required)) = doc.meta.get("requires") {
        for item in required {
            if let Value::String(feature) = item {
                validate_required_profile(feature, profiles, diagnostics);
            }
        }
    }

    if let Some(Value::Map(map)) = doc.meta.get("profiles") {
        if let Some(Value::List(required)) = map.get("requires") {
            for item in required {
                if let Value::String(profile) = item {
                    validate_required_profile(profile, profiles, diagnostics);
                }
            }
        }
        if let Some(Value::List(optional)) = map.get("optional") {
            for item in optional {
                if let Value::String(profile) = item {
                    if !profiles.supports(profile) {
                        diagnostics.push(Diagnostic {
                            code: "NODX-E023".to_string(),
                            severity: "warning".to_string(),
                            message: format!("Optional profile `{}` is unsupported.", profile),
                            line: None,
                            column: None,
                            target: Some(format!("profile:{profile}")),
                        });
                    }
                }
            }
        }
    }
}

fn validate_required_profile(
    profile: &str,
    profiles: &ProfileSet,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !profiles.supports(profile) {
        diagnostics.push(Diagnostic {
            code: "NODX-E024".to_string(),
            severity: "error".to_string(),
            message: format!("Required profile `{}` is unsupported.", profile),
            line: None,
            column: None,
            target: Some(format!("profile:{profile}")),
        });
    }
}

fn legacy_supported_feature(feature: &str) -> bool {
    matches!(
        feature,
        "rich-tables" | "math" | "media" | "custom-components"
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
    limits: ResourceLimits,
) {
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
            "image" => validate_image(node, diagnostics, limits),
            "media" | "embed" | "include" => validate_asset_node(node, diagnostics, limits),
            "table" => validate_table(node, diagnostics),
            "toc" => validate_toc(node, diagnostics),
            _ => {}
        }
        collect_inline_refs(&node.inlines, refs, vars, diagnostics, limits);
        validate_nodes(
            &node.children,
            ids,
            refs,
            components,
            vars,
            previous_heading,
            diagnostics,
            limits,
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

fn validate_image(node: &Node, diagnostics: &mut Vec<Diagnostic>, limits: ResourceLimits) {
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
    validate_asset_node(node, diagnostics, limits);
}

fn validate_asset_node(node: &Node, diagnostics: &mut Vec<Diagnostic>, limits: ResourceLimits) {
    if let Some(src) = node.attrs.get("src") {
        if ResourcePolicy::new(limits)
            .classify_uri(ReferenceKind::Asset, src)
            .is_err()
        {
            diagnostics.push(validation_diag(
                "NODX-E008",
                "error",
                "Unresolvable asset.",
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

fn validate_toc(node: &Node, diagnostics: &mut Vec<Diagnostic>) {
    if let Some(role) = node.attrs.get("role") {
        if !matches!(
            role.as_str(),
            "primary" | "local" | "secondary" | "breadcrumb"
        ) {
            diagnostics.push(validation_diag(
                "NODX-E004",
                "error",
                "Invalid toc role.",
                role,
            ));
        }
    }
    if let Some(source) = node.attrs.get("source") {
        if source != "document" {
            diagnostics.push(validation_diag(
                "NODX-E004",
                "error",
                "Invalid toc source.",
                source,
            ));
        }
    }
    if let Some(scope) = node.attrs.get("scope") {
        if !scope.starts_with('#') || scope.len() == 1 {
            diagnostics.push(validation_diag(
                "NODX-E004",
                "error",
                "Invalid toc scope.",
                scope,
            ));
        }
    }
    for name in ["depth", "min-level", "max-level"] {
        if let Some(value) = node.attrs.get(name) {
            if parse_level(value).is_none() {
                diagnostics.push(validation_diag(
                    "NODX-E004",
                    "error",
                    "Invalid toc level attribute.",
                    value,
                ));
            }
        }
    }
    if let (Some(min), Some(max)) = (
        node.attrs.get("min-level").and_then(|v| parse_level(v)),
        node.attrs.get("max-level").and_then(|v| parse_level(v)),
    ) {
        if min > max {
            diagnostics.push(validation_diag(
                "NODX-E004",
                "error",
                "toc min-level must not exceed max-level.",
                &format!("{min}..{max}"),
            ));
        }
    }
    if !node.attrs.contains_key("title") {
        let role = node
            .attrs
            .get("role")
            .map(String::as_str)
            .unwrap_or("primary");
        let label = default_navigation_label(role);
        diagnostics.push(validation_diag(
            "NODX-E016",
            "info",
            "toc title omitted; deterministic accessible label will be used.",
            label,
        ));
    }
}

fn validate_navigation(doc: &Document, ids: &BTreeSet<String>, diagnostics: &mut Vec<Diagnostic>) {
    let _graph = resolve_navigation(doc);
    collect_toc_scopes(&doc.body, ids, diagnostics);
}

fn collect_toc_scopes(nodes: &[Node], ids: &BTreeSet<String>, diagnostics: &mut Vec<Diagnostic>) {
    for node in nodes {
        if node.node_type == "toc" {
            if let Some(scope) = node.attrs.get("scope") {
                if let Some(id) = scope.strip_prefix('#') {
                    if !ids.contains(id) {
                        diagnostics.push(validation_diag(
                            "NODX-E007",
                            "error",
                            "Unresolved toc scope.",
                            scope,
                        ));
                    }
                }
            }
        }
        collect_toc_scopes(&node.children, ids, diagnostics);
    }
}

fn parse_level(value: &str) -> Option<usize> {
    value
        .parse::<usize>()
        .ok()
        .filter(|level| (1..=6).contains(level))
}

fn collect_inline_refs(
    inlines: &[Inline],
    refs: &mut Vec<String>,
    vars: &BTreeSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
    limits: ResourceLimits,
) {
    for item in inlines {
        match item {
            Inline::Strong(children)
            | Inline::Em(children)
            | Inline::Mark(children)
            | Inline::Sub(children)
            | Inline::Sup(children) => {
                collect_inline_refs(children, refs, vars, diagnostics, limits)
            }
            Inline::Link { label, target } => {
                if ResourcePolicy::new(limits)
                    .classify_uri(ReferenceKind::Link, target)
                    .is_err()
                {
                    diagnostics.push(validation_diag(
                        "NODX-E020",
                        "error",
                        "Unsafe URL or scheme.",
                        target,
                    ));
                }
                collect_inline_refs(label, refs, vars, diagnostics, limits);
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
                collect_inline_refs(children, refs, vars, diagnostics, limits);
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

fn write_json_option_usize(out: &mut String, value: Option<usize>) {
    match value {
        Some(value) => out.push_str(&value.to_string()),
        None => out.push_str("null"),
    }
}

fn write_json_string(out: &mut String, input: &str) {
    out.push('"');
    for ch in input.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use nodx_core::parse_str;

    use super::{Validator, diagnostics_json, exit_code_for};

    #[test]
    fn unsupported_required_profile_exits_three() {
        let doc =
            parse_str("---\nschema: nodx/1.0\nprofiles:\n  requires: [signature]\n---\n\n# A\n");
        let diagnostics = Validator::default().validate(&doc);
        assert!(diagnostics.iter().any(|d| d.code == "NODX-E024"));
        assert_eq!(exit_code_for(&diagnostics), 3);
    }

    #[test]
    fn unsupported_optional_profile_warns() {
        let doc =
            parse_str("---\nschema: nodx/1.0\nprofiles:\n  optional: [signature]\n---\n\n# A\n");
        let diagnostics = Validator::default().validate(&doc);
        assert!(diagnostics.iter().any(|d| d.code == "NODX-E023"));
        assert_eq!(exit_code_for(&diagnostics), 0);
    }

    #[test]
    fn validates_navigation_attributes_and_scope() {
        let doc = parse_str(
            ":::toc {role=\"bad\" scope=\"#missing\" depth=\"7\" min-level=\"4\" max-level=\"2\"}\n:::\n",
        );
        let diagnostics = Validator::default().validate(&doc);
        assert!(diagnostics.iter().any(|d| d.code == "NODX-E004"));
        assert!(diagnostics.iter().any(|d| d.code == "NODX-E007"));
    }

    #[test]
    fn diagnostic_json_shape_is_stable() {
        let doc =
            parse_str("---\nschema: nodx/1.0\nprofiles:\n  requires: [signature]\n---\n\n# A\n");
        let diagnostics = Validator::default().validate(&doc);
        let json = diagnostics_json(&diagnostics);
        assert!(json.contains("\"code\":\"NODX-E024\""));
        assert!(json.contains("\"line\":null"));
        assert!(json.contains("\"target\":\"profile:signature\""));
    }

    #[test]
    fn negative_fixtures_match_golden_diagnostics() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for name in [
            "e004-invalid-schema-and-toc",
            "e006-duplicate-id",
            "e007-unresolved-reference-and-toc-scope",
            "e008-unresolvable-asset",
            "e009-missing-alt",
            "e013-undeclared-var",
            "e014-custom-component",
            "e016-toc-default-label",
            "e022-heading-jump",
            "e023-unsupported-optional-profile",
            "e024-unsupported-required-profile",
            "e025-invalid-table-grid",
        ] {
            let source = fs::read_to_string(
                root.join("spec/tests/negative")
                    .join(format!("{name}.nodx")),
            )
            .expect("read negative fixture");
            let expected = fs::read_to_string(
                root.join("spec/tests/golden")
                    .join(format!("{name}.diagnostics.json")),
            )
            .expect("read golden diagnostics");
            let doc = parse_str(&source);
            let diagnostics = Validator::default().validate(&doc);
            assert_eq!(
                diagnostics_json(&diagnostics),
                expected.trim_end(),
                "{name}"
            );
        }
    }
}

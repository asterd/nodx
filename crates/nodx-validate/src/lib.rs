#![forbid(unsafe_code)]

use std::collections::BTreeSet;

use nodx_core::{
    Diagnostic, Document, Inline, Node, ResourceLimits, Value, canonical_json,
    default_navigation_label, resolve_navigation, sha256_base64url, valid_name,
};
use nodx_style::{audit_stylesheet, yaml_style_to_css};
use nodx_url::{ReferenceKind, ResourcePolicy};

pub const SCHEMA_1_0: &str = "nodx/1.0";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileSet {
    supported: BTreeSet<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Validator {
    profiles: ProfileSet,
    limits: ResourceLimits,
}

impl Default for ProfileSet {
    fn default() -> Self {
        Self::new([
            "plain",
            "core",
            "rich",
            "style",
            "package",
            "agent-read",
            "remote-assets",
        ])
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
        self.supported.contains(profile)
    }

    pub fn deferred(profile: &str) -> bool {
        matches!(
            profile,
            "agent-mutate" | "signature" | "editor" | "presentation"
        )
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
        validate_integrity(doc, diagnostics);
        let declared_components = component_names(doc);
        let declared_vars = declared_vars(doc);
        let allow_remote_assets = document_allows_remote_assets(doc);
        let mut state = NodeValidationState {
            ids: BTreeSet::new(),
            refs: Vec::new(),
            previous_heading: 0,
            diagnostics,
        };
        validate_nodes(
            &doc.body,
            &declared_components,
            &declared_vars,
            self.limits,
            allow_remote_assets,
            &mut state,
        );
        validate_navigation(doc, &state.ids, state.diagnostics);
        for target in state.refs {
            if !state.ids.contains(&target) {
                state.diagnostics.push(Diagnostic {
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
    let has_unsupported_required = diagnostics
        .iter()
        .any(|d| d.code == "NODX-E024" && (d.severity == "fatal" || d.severity == "error"));
    let has_error = diagnostics
        .iter()
        .any(|d| d.severity == "fatal" || d.severity == "error");
    if has_unsupported_required {
        3
    } else if has_error {
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
        Some(Value::String(schema)) if schema == SCHEMA_1_0 => {}
        Some(Value::String(other)) => diagnostics.push(Diagnostic {
            code: "NODX-E004".to_string(),
            severity: "error".to_string(),
            message: format!(
                "Schema `{other}` is not the frozen NODX 1.0 contract `{SCHEMA_1_0}`."
            ),
            line: None,
            column: None,
            target: Some("schema".to_string()),
        }),
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

    if let Some(Value::String(theme)) = doc.meta.get("theme")
        && !is_valid_theme(theme)
    {
        diagnostics.push(Diagnostic {
            code: "NODX-E004".to_string(),
            severity: "error".to_string(),
            message: "Invalid theme name or path.".to_string(),
            line: None,
            column: None,
            target: Some("theme".to_string()),
        });
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
                if let Value::String(profile) = item
                    && !profiles.supports(profile)
                {
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

fn is_valid_theme(theme: &str) -> bool {
    matches!(
        theme,
        "none" | "plain" | "base" | "web" | "print" | "presentation" | "docs"
    ) || (theme.ends_with(".nodt")
        && !theme.starts_with('/')
        && !theme.contains("..")
        && !theme.contains('\\'))
}

pub fn document_allows_remote_assets(doc: &Document) -> bool {
    if let Some(Value::Map(features)) = doc.meta.get("features")
        && matches!(features.get("remote-assets"), Some(Value::Bool(true)))
    {
        return true;
    }
    if let Some(Value::Map(profiles)) = doc.meta.get("profiles") {
        for key in ["requires", "optional"] {
            if let Some(Value::List(items)) = profiles.get(key)
                && items.iter().any(
                    |item| matches!(item, Value::String(profile) if profile == "remote-assets"),
                )
            {
                return true;
            }
        }
    }
    false
}

fn validate_integrity(doc: &Document, diagnostics: &mut Vec<Diagnostic>) {
    let Some(Value::Map(integrity)) = doc.meta.get("integrity") else {
        return;
    };
    let alg = match integrity.get("alg") {
        Some(Value::String(alg)) => alg.as_str(),
        _ => "",
    };
    let scope = match integrity.get("scope") {
        Some(Value::String(scope)) => scope.as_str(),
        _ => "canonical-ast",
    };
    let value = match integrity.get("value") {
        Some(Value::String(value)) => value.as_str(),
        _ => "",
    };
    if alg != "sha256" || scope != "canonical-ast" || !value.starts_with("sha256-") {
        diagnostics.push(validation_diag(
            "NODX-E028",
            "error",
            "Invalid integrity declaration.",
            "integrity",
        ));
        return;
    }
    let expected = integrity_digest(doc);
    if value != expected {
        diagnostics.push(validation_diag(
            "NODX-E028",
            "error",
            "Document integrity digest does not match.",
            "integrity",
        ));
    }
}

pub fn integrity_digest(doc: &Document) -> String {
    let mut unsigned = doc.clone();
    unsigned.meta.remove("integrity");
    sha256_base64url(canonical_json(&unsigned).as_bytes())
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

fn component_names(doc: &Document) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    if let Some(Value::List(items)) = doc.meta.get("components") {
        for item in items {
            if let Value::Map(map) = item
                && let Some(Value::String(name)) = map.get("name")
            {
                out.insert(name.clone());
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

struct NodeValidationState<'a> {
    ids: BTreeSet<String>,
    refs: Vec<String>,
    previous_heading: usize,
    diagnostics: &'a mut Vec<Diagnostic>,
}

fn validate_nodes(
    nodes: &[Node],
    components: &BTreeSet<String>,
    vars: &BTreeSet<String>,
    limits: ResourceLimits,
    allow_remote_assets: bool,
    state: &mut NodeValidationState<'_>,
) {
    for node in nodes {
        if let Some(id) = &node.id {
            if !valid_name(id, true) || id.len() > limits.id_bytes {
                state.diagnostics.push(validation_diag(
                    "NODX-E004",
                    "error",
                    "Invalid node id.",
                    id,
                ));
            }
            if !state.ids.insert(id.clone()) {
                state.diagnostics.push(validation_diag(
                    "NODX-E006",
                    "error",
                    "Duplicate node id.",
                    id,
                ));
            }
        }
        validate_common_attrs(node, state.diagnostics);
        if node.node_type.contains('-')
            && !is_standard_node(&node.node_type)
            && !components.contains(&node.node_type)
            && !node.attrs.contains_key("fallback")
        {
            state.diagnostics.push(validation_diag(
                "NODX-E014",
                "warning",
                "Custom component is not declared and has no explicit fallback.",
                &node.node_type,
            ));
        }
        match node.node_type.as_str() {
            "heading" => validate_heading(node, &mut state.previous_heading, state.diagnostics),
            "image" => validate_image(node, state.diagnostics, limits, allow_remote_assets),
            "media" | "embed" => validate_asset_node(
                node,
                state.diagnostics,
                limits,
                allow_remote_assets,
                ReferenceKind::MediaFallback,
            ),
            "include" => validate_asset_node(
                node,
                state.diagnostics,
                limits,
                false,
                ReferenceKind::Include,
            ),
            "table" => validate_table(node, state.diagnostics),
            "toc" => validate_toc(node, state.diagnostics),
            "style" => validate_style_block(node, state.diagnostics, limits),
            _ => {}
        }
        collect_inline_refs(
            &node.inlines,
            &mut state.refs,
            vars,
            state.diagnostics,
            limits,
        );
        validate_nodes(
            &node.children,
            components,
            vars,
            limits,
            allow_remote_assets,
            state,
        );
    }
}

fn validate_common_attrs(node: &Node, diagnostics: &mut Vec<Diagnostic>) {
    if let Some(dir) = node.attrs.get("dir")
        && !matches!(dir.as_str(), "ltr" | "rtl" | "auto")
    {
        diagnostics.push(validation_diag(
            "NODX-E004",
            "error",
            "Invalid dir attribute.",
            dir,
        ));
    }
}

fn is_standard_node(node_type: &str) -> bool {
    matches!(
        node_type,
        "citation-entry" | "pagebreak" | "speaker-notes" | "media-fallback"
    )
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

fn validate_image(
    node: &Node,
    diagnostics: &mut Vec<Diagnostic>,
    limits: ResourceLimits,
    allow_remote_assets: bool,
) {
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
    validate_asset_node(
        node,
        diagnostics,
        limits,
        allow_remote_assets,
        ReferenceKind::Asset,
    );
}

fn validate_asset_node(
    node: &Node,
    diagnostics: &mut Vec<Diagnostic>,
    limits: ResourceLimits,
    allow_remote_assets: bool,
    kind: ReferenceKind,
) {
    if let Some(src) = node.attrs.get("src")
        && ResourcePolicy::new(limits)
            .with_remote_assets(allow_remote_assets)
            .classify_uri(kind, src)
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
            .map(cell_width)
            .sum::<usize>();
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

fn cell_width(cell: &Node) -> usize {
    cell.attrs
        .get("colspan")
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(1)
}

fn validate_toc(node: &Node, diagnostics: &mut Vec<Diagnostic>) {
    if let Some(role) = node.attrs.get("role")
        && !matches!(
            role.as_str(),
            "primary" | "local" | "secondary" | "breadcrumb"
        )
    {
        diagnostics.push(validation_diag(
            "NODX-E004",
            "error",
            "Invalid toc role.",
            role,
        ));
    }
    if let Some(source) = node.attrs.get("source")
        && source != "document"
    {
        diagnostics.push(validation_diag(
            "NODX-E004",
            "error",
            "Invalid toc source.",
            source,
        ));
    }
    if let Some(mode) = node.attrs.get("mode")
        && !matches!(mode.as_str(), "auto" | "manual")
    {
        diagnostics.push(validation_diag(
            "NODX-E004",
            "error",
            "Invalid toc mode.",
            mode,
        ));
    }
    if let Some(scope) = node.attrs.get("scope")
        && (!scope.starts_with('#') || scope.len() == 1)
    {
        diagnostics.push(validation_diag(
            "NODX-E004",
            "error",
            "Invalid toc scope.",
            scope,
        ));
    }
    for name in ["depth", "min-level", "max-level"] {
        if let Some(value) = node.attrs.get(name)
            && parse_level(value).is_none()
        {
            diagnostics.push(validation_diag(
                "NODX-E004",
                "error",
                "Invalid toc level attribute.",
                value,
            ));
        }
    }
    if let (Some(min), Some(max)) = (
        node.attrs.get("min-level").and_then(|v| parse_level(v)),
        node.attrs.get("max-level").and_then(|v| parse_level(v)),
    ) && min > max
    {
        diagnostics.push(validation_diag(
            "NODX-E004",
            "error",
            "toc min-level must not exceed max-level.",
            &format!("{min}..{max}"),
        ));
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

fn validate_style_block(node: &Node, diagnostics: &mut Vec<Diagnostic>, limits: ResourceLimits) {
    let Some(text) = &node.text else {
        return;
    };
    let style_source;
    let source = if node.attrs.get("format").map(String::as_str) == Some("yaml") {
        match yaml_style_to_css(text) {
            Ok(css) => {
                style_source = css;
                style_source.as_str()
            }
            Err(message) => {
                diagnostics.push(Diagnostic {
                    code: "NODX-E027".to_string(),
                    severity: "error".to_string(),
                    message,
                    line: None,
                    column: None,
                    target: node.id.clone(),
                });
                return;
            }
        }
    } else {
        text.as_str()
    };
    let audit = audit_stylesheet(source, limits);
    for violation in audit.violations {
        diagnostics.push(Diagnostic {
            code: "NODX-E027".to_string(),
            severity: violation.severity.to_string(),
            message: format!(
                "{} `{}` in :::style block.",
                violation.message, violation.construct
            ),
            line: None,
            column: None,
            target: node.id.clone(),
        });
    }
}

fn validate_navigation(doc: &Document, ids: &BTreeSet<String>, diagnostics: &mut Vec<Diagnostic>) {
    let _graph = resolve_navigation(doc);
    collect_toc_scopes(&doc.body, ids, diagnostics);
}

fn collect_toc_scopes(nodes: &[Node], ids: &BTreeSet<String>, diagnostics: &mut Vec<Diagnostic>) {
    for node in nodes {
        if node.node_type == "toc"
            && let Some(scope) = node.attrs.get("scope")
            && let Some(id) = scope.strip_prefix('#')
            && !ids.contains(id)
        {
            diagnostics.push(validation_diag(
                "NODX-E007",
                "error",
                "Unresolved toc scope.",
                scope,
            ));
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
            | Inline::Strike(children)
            | Inline::Sub(children)
            | Inline::Sup(children) => {
                collect_inline_refs(children, refs, vars, diagnostics, limits)
            }
            Inline::Mark { children, attrs } => {
                validate_inline_attrs(attrs, diagnostics);
                collect_inline_refs(children, refs, vars, diagnostics, limits);
            }
            Inline::Link {
                label,
                target,
                attrs,
            } => {
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
                validate_inline_attrs(attrs, diagnostics);
                collect_inline_refs(label, refs, vars, diagnostics, limits);
            }
            Inline::Span { children, attrs } => {
                validate_inline_attrs(attrs, diagnostics);
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
            | Inline::MathInline { .. }
            | Inline::LineBreak => {}
        }
    }
}

fn validate_inline_attrs(attrs: &nodx_core::Attrs, diagnostics: &mut Vec<Diagnostic>) {
    if let Some(dir) = attrs.attrs.get("dir")
        && !matches!(dir.as_str(), "ltr" | "rtl" | "auto")
    {
        diagnostics.push(validation_diag(
            "NODX-E004",
            "error",
            "Invalid inline dir attribute.",
            dir,
        ));
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

    use super::{ProfileSet, Validator, diagnostics_json, exit_code_for, integrity_digest};

    #[test]
    fn unsupported_required_profile_exits_three() {
        let doc = parse_str(
            "---\nschema: nodx/1.0\nprofiles:\n  requires:\n    - signature\n---\n\n# A\n",
        );
        let diagnostics = Validator::default().validate(&doc);
        assert!(diagnostics.iter().any(|d| d.code == "NODX-E024"));
        assert_eq!(exit_code_for(&diagnostics), 3);
    }

    #[test]
    fn deferred_presentation_required_profile_exits_three() {
        let doc = parse_str(
            "---\nschema: nodx/1.0\nprofiles:\n  requires:\n    - presentation\n---\n\n# A\n",
        );
        let diagnostics = Validator::default().validate(&doc);
        assert!(diagnostics.iter().any(|d| d.code == "NODX-E024"));
        assert_eq!(exit_code_for(&diagnostics), 3);
    }

    #[test]
    fn legacy_schema_zero_one_is_rejected() {
        let doc = parse_str("---\nschema: nodx/0.1\n---\n# A\n");
        let diagnostics = Validator::default().validate(&doc);
        assert!(diagnostics.iter().any(|d| d.code == "NODX-E004"));
    }

    #[test]
    fn unsupported_optional_profile_warns() {
        let doc = parse_str(
            "---\nschema: nodx/1.0\nprofiles:\n  optional:\n    - signature\n---\n\n# A\n",
        );
        let diagnostics = Validator::default().validate(&doc);
        assert!(diagnostics.iter().any(|d| d.code == "NODX-E023"));
        assert_eq!(exit_code_for(&diagnostics), 0);
    }

    #[test]
    fn remote_assets_profile_allows_remote_media() {
        let doc = parse_str(
            "---\nschema: nodx/1.0\nprofiles:\n  optional:\n    - remote-assets\n---\n\n:::image {src=\"https://example.test/a.png\" alt=\"A\"}\n:::\n\n:::media {src=\"https://example.test/a.mp4\" alt=\"A\"}\n:::\n",
        );
        let diagnostics = Validator::default().validate(&doc);
        assert!(!diagnostics.iter().any(|d| d.code == "NODX-E008"));
    }

    #[test]
    fn integrity_digest_is_verified_excluding_integrity_header() {
        let mut doc = parse_str("---\nschema: nodx/1.0\n---\n\n# A\n");
        let digest = integrity_digest(&doc);
        let mut integrity = std::collections::BTreeMap::new();
        integrity.insert(
            "alg".to_string(),
            nodx_core::Value::String("sha256".to_string()),
        );
        integrity.insert(
            "scope".to_string(),
            nodx_core::Value::String("canonical-ast".to_string()),
        );
        integrity.insert("value".to_string(), nodx_core::Value::String(digest));
        doc.meta
            .insert("integrity".to_string(), nodx_core::Value::Map(integrity));
        assert!(Validator::default().validate(&doc).is_empty());
        doc.body[0]
            .attrs
            .insert("class".to_string(), "changed".to_string());
        let diagnostics = Validator::default().validate(&doc);
        assert!(diagnostics.iter().any(|d| d.code == "NODX-E028"));
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
    fn nods_audit_runs_in_validator() {
        let doc = parse_str("---\nschema: nodx/1.0\n---\n:::style\na:hover { color: red; }\n:::\n");
        let diagnostics = Validator::default().validate(&doc);
        assert!(diagnostics.iter().any(|d| d.code == "NODX-E027"));
    }

    #[test]
    fn deferred_profile_helper() {
        assert!(ProfileSet::deferred("presentation"));
        assert!(ProfileSet::deferred("signature"));
        assert!(!ProfileSet::deferred("rich"));
    }

    #[test]
    fn diagnostic_json_shape_is_stable() {
        let doc = parse_str(
            "---\nschema: nodx/1.0\nprofiles:\n  requires:\n    - signature\n---\n\n# A\n",
        );
        let diagnostics = Validator::default().validate(&doc);
        let json = diagnostics_json(&diagnostics);
        assert!(json.contains("\"code\":\"NODX-E024\""));
        assert!(json.contains("\"line\":null"));
        assert!(json.contains("\"target\":\"profile:signature\""));
    }

    #[test]
    fn negative_corpus_size_meets_release_target() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let dir = root.join("spec/tests/negative");
        let count = fs::read_dir(&dir)
            .expect("negative dir")
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("nodx"))
            .count();
        assert!(
            count >= 50,
            "expected at least 50 negative fixtures, found {count}"
        );
    }

    #[test]
    fn negative_corpus_emits_expected_code_class() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let dir = root.join("spec/tests/negative");
        for entry in fs::read_dir(&dir).expect("negative dir") {
            let entry = entry.expect("dirent");
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if !name.ends_with(".nodx") {
                continue;
            }
            let stem = name.strip_suffix(".nodx").unwrap();
            let expected_code = stem
                .strip_prefix("e004")
                .map(|_| "NODX-E004")
                .or_else(|| stem.strip_prefix("e006").map(|_| "NODX-E006"))
                .or_else(|| stem.strip_prefix("e007").map(|_| "NODX-E007"))
                .or_else(|| stem.strip_prefix("e008").map(|_| "NODX-E008"))
                .or_else(|| stem.strip_prefix("e009").map(|_| "NODX-E009"))
                .or_else(|| stem.strip_prefix("e013").map(|_| "NODX-E013"))
                .or_else(|| stem.strip_prefix("e014").map(|_| "NODX-E014"))
                .or_else(|| stem.strip_prefix("e016").map(|_| "NODX-E016"))
                .or_else(|| stem.strip_prefix("e020").map(|_| "NODX-E020"))
                .or_else(|| stem.strip_prefix("e022").map(|_| "NODX-E022"))
                .or_else(|| stem.strip_prefix("e023").map(|_| "NODX-E023"))
                .or_else(|| stem.strip_prefix("e024").map(|_| "NODX-E024"))
                .or_else(|| stem.strip_prefix("e025").map(|_| "NODX-E025"));
            let Some(expected_code) = expected_code else {
                continue;
            };
            let source = fs::read_to_string(&path).expect("read fixture");
            let doc = parse_str(&source);
            let diagnostics = Validator::default().validate(&doc);
            assert!(
                diagnostics.iter().any(|d| d.code == expected_code),
                "{name} should emit {expected_code}; got {:?}",
                diagnostics
                    .iter()
                    .map(|d| d.code.as_str())
                    .collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn negative_fixtures_match_golden_diagnostics() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let golden_dir = root.join("spec/tests/golden");
        for entry in fs::read_dir(&golden_dir).expect("read golden dir") {
            let entry = entry.expect("dirent");
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let Some(stem) = name.strip_suffix(".diagnostics.json") else {
                continue;
            };
            let source = fs::read_to_string(
                root.join("spec/tests/negative")
                    .join(format!("{stem}.nodx")),
            )
            .unwrap_or_else(|_| panic!("missing negative source for {stem}"));
            let expected = fs::read_to_string(entry.path())
                .unwrap_or_else(|_| panic!("missing golden for {stem}"));
            let doc = parse_str(&source);
            let diagnostics = Validator::default().validate(&doc);
            assert_eq!(
                diagnostics_json(&diagnostics),
                expected.trim_end(),
                "{stem}"
            );
        }
    }
}

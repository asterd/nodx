use crate::ResourceLimits;
use crate::ast::Node;
use crate::diagnostic::Diagnostic;
use nodx_url::{ReferenceKind, ResourcePolicy};

const FORBIDDEN_NODS_PATTERNS: &[&str] = &[
    ":hover",
    ":focus",
    ":active",
    ":visited",
    ":checked",
    ":target",
    "::before",
    "::after",
    "::first-line",
    "::first-letter",
    "::placeholder",
    ":nth-child",
    ":nth-of-type",
    ":first-child",
    ":last-child",
    ":not(",
    ":is(",
    ":where(",
    ":has(",
    "@keyframes",
    "@supports",
    "@container",
    "@layer",
    "@property",
    "@scope",
    "position:fixed",
    "position: fixed",
    "position:sticky",
    "position: sticky",
    "transform:",
    "transform ",
    "transition:",
    "transition ",
    "animation:",
    "animation ",
    "will-change",
    "cursor:",
    "pointer-events",
    "user-select",
    "clip-path",
    "backdrop-filter",
    "attr(",
    "env(",
    "counter(",
    "expression(",
];

pub(crate) fn audit_nods(
    nodes: &[Node],
    diagnostics: &mut Vec<Diagnostic>,
    limits: ResourceLimits,
) {
    let policy = ResourcePolicy::new(limits);
    for node in nodes {
        if node.node_type == "style" {
            if let Some(text) = &node.text {
                let lower = text.to_ascii_lowercase();
                for pattern in FORBIDDEN_NODS_PATTERNS {
                    if lower.contains(pattern) {
                        diagnostics.push(Diagnostic {
                            code: "NODX-E027".to_string(),
                            severity: "warning".to_string(),
                            message: format!(
                                "Forbidden NODS construct `{}` in :::style block.",
                                pattern.trim()
                            ),
                            line: None,
                            column: None,
                            target: node.id.clone(),
                        });
                    }
                }
                for url in style_urls(text) {
                    if policy.classify_uri(ReferenceKind::Style, url).is_err() {
                        diagnostics.push(Diagnostic {
                            code: "NODX-E020".to_string(),
                            severity: "error".to_string(),
                            message: "Unsafe URL or scheme.".to_string(),
                            line: None,
                            column: None,
                            target: node.id.clone(),
                        });
                    }
                }
            }
        }
        audit_nods(&node.children, diagnostics, limits);
    }
}

pub(crate) fn strip_forbidden_nods(input: &str) -> String {
    if FORBIDDEN_NODS_PATTERNS
        .iter()
        .all(|p| !input.to_ascii_lowercase().contains(p))
    {
        return input.to_string();
    }
    let mut out = String::with_capacity(input.len());
    for line in input.split_inclusive('\n') {
        let lower = line.to_ascii_lowercase();
        if FORBIDDEN_NODS_PATTERNS.iter().any(|p| lower.contains(p)) {
            out.push_str("/* nodx-E027: forbidden NODS rule omitted */");
            if line.ends_with('\n') {
                out.push('\n');
            }
        } else {
            out.push_str(line);
        }
    }
    out
}

pub(crate) fn style_urls(input: &str) -> Vec<&str> {
    let mut urls = Vec::new();
    let mut offset = 0;
    let lower = input.to_ascii_lowercase();
    while let Some(start) = lower[offset..].find("url(") {
        let url_start = offset + start + 4;
        let after = &input[url_start..];
        let Some(end) = after.find(')') else {
            break;
        };
        let raw = after[..end].trim().trim_matches('"').trim_matches('\'');
        urls.push(raw);
        offset = url_start + end + 1;
    }
    urls
}

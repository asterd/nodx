use crate::ast::Node;
use crate::diagnostic::Diagnostic;

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

pub(crate) fn audit_nods(nodes: &[Node], diagnostics: &mut Vec<Diagnostic>) {
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
            }
        }
        audit_nods(&node.children, diagnostics);
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

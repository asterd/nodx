use crate::ResourceLimits;
use crate::ast::Node;
use crate::diagnostic::Diagnostic;

pub(crate) fn audit_nods(
    nodes: &[Node],
    diagnostics: &mut Vec<Diagnostic>,
    limits: ResourceLimits,
) {
    for node in nodes {
        if node.node_type == "style" {
            if let Some(text) = &node.text {
                let audit = nodx_style::audit_stylesheet(text, limits);
                for violation in audit.violations {
                    diagnostics.push(Diagnostic {
                        code: "NODX-E027".to_string(),
                        severity: "warning".to_string(),
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
        }
        audit_nods(&node.children, diagnostics, limits);
    }
}

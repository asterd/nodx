#![forbid(unsafe_code)]

use std::collections::BTreeSet;

use nodx_url::{ReferenceKind, ResourceLimits, ResourcePolicy};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StyleViolation {
    pub construct: String,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StyleAudit {
    pub violations: Vec<StyleViolation>,
}

impl StyleAudit {
    pub fn is_safe(&self) -> bool {
        self.violations.is_empty()
    }
}

pub fn audit_stylesheet(input: &str, limits: ResourceLimits) -> StyleAudit {
    let mut audit = StyleAudit {
        violations: Vec::new(),
    };
    audit_breakouts(input, &mut audit);
    for rule in parse_rules(input) {
        audit_rule(rule, limits, &mut audit);
    }
    dedupe(&mut audit.violations);
    audit
}

pub fn sanitize_stylesheet(input: &str, limits: ResourceLimits) -> String {
    let audit = audit_stylesheet(input, limits);
    if audit.is_safe() {
        return escape_style_text(input);
    }
    if audit
        .violations
        .iter()
        .any(|v| v.message == "Forbidden executable or breakout content in NODS.")
    {
        return "/* NODX-E027: blocked unsafe style content */".to_string();
    }

    let mut out = String::new();
    for rule in parse_rules(input) {
        let source = rule.source.trim();
        if source.is_empty() {
            continue;
        }
        if audit_rule_to_vec(rule, limits).is_empty() {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(&escape_style_text(source));
        } else {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str("/* NODX-E027: forbidden NODS rule omitted */");
        }
    }
    if out.is_empty() {
        "/* NODX-E027: blocked unsafe style content */".to_string()
    } else {
        out
    }
}

pub fn style_urls(input: &str) -> Vec<&str> {
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

#[derive(Clone, Copy)]
struct Rule<'a> {
    prelude: &'a str,
    declarations: &'a str,
    source: &'a str,
}

fn parse_rules(input: &str) -> Vec<Rule<'_>> {
    let mut rules = Vec::new();
    let bytes = input.as_bytes();
    let mut start = 0usize;
    while start < input.len() {
        let Some(open_rel) = input[start..].find('{') else {
            break;
        };
        let open = start + open_rel;
        let mut depth = 1usize;
        let mut i = open + 1;
        while i < bytes.len() {
            match bytes[i] {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            i += 1;
        }
        if depth != 0 {
            rules.push(Rule {
                prelude: &input[start..open],
                declarations: &input[open + 1..],
                source: &input[start..],
            });
            break;
        }
        rules.push(Rule {
            prelude: &input[start..open],
            declarations: &input[open + 1..i],
            source: &input[start..=i],
        });
        start = i + 1;
    }
    rules
}

fn audit_breakouts(input: &str, audit: &mut StyleAudit) {
    let lower = input.to_ascii_lowercase();
    for construct in ["</style", "<script", "<svg", "<iframe", "<object", "<embed"] {
        if lower.contains(construct) {
            audit.violations.push(violation(
                construct,
                "Forbidden executable or breakout content in NODS.",
            ));
        }
    }
}

fn audit_rule(rule: Rule<'_>, limits: ResourceLimits, audit: &mut StyleAudit) {
    audit.violations.extend(audit_rule_to_vec(rule, limits));
}

fn audit_rule_to_vec(rule: Rule<'_>, limits: ResourceLimits) -> Vec<StyleViolation> {
    let mut violations = Vec::new();
    let prelude = rule.prelude.trim();
    if prelude.starts_with('@') {
        audit_at_rule(prelude, &mut violations);
    } else {
        audit_selector(prelude, &mut violations);
    }
    audit_declarations(rule.declarations, limits, &mut violations);
    dedupe(&mut violations);
    violations
}

fn audit_at_rule(prelude: &str, violations: &mut Vec<StyleViolation>) {
    let lower = prelude.to_ascii_lowercase();
    if lower.starts_with("@page") {
        return;
    }
    violations.push(violation(prelude, "Forbidden NODS at-rule."));
}

fn audit_selector(selector: &str, violations: &mut Vec<StyleViolation>) {
    if selector.is_empty() {
        violations.push(violation("selector", "Missing NODS selector."));
        return;
    }
    for forbidden in [
        ":", "::", "[", "]", ">", "+", "~", "*", "|", "$", "^", "=", "!",
    ] {
        if selector.contains(forbidden) {
            violations.push(violation(selector, "Forbidden NODS selector."));
            return;
        }
    }
    for part in selector.split(',') {
        for token in part.split_ascii_whitespace() {
            if !is_safe_selector_token(token) {
                violations.push(violation(token, "Unsupported NODS selector."));
            }
        }
    }
}

fn is_safe_selector_token(token: &str) -> bool {
    if token.is_empty() {
        return false;
    }
    if let Some(class) = token.strip_prefix('.') {
        return is_ident(class);
    }
    if let Some(id) = token.strip_prefix('#') {
        return is_ident(id);
    }
    matches!(
        token,
        "a" | "article"
            | "aside"
            | "blockquote"
            | "body"
            | "caption"
            | "code"
            | "dd"
            | "div"
            | "dl"
            | "dt"
            | "em"
            | "figcaption"
            | "figure"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "hr"
            | "html"
            | "img"
            | "li"
            | "mark"
            | "nav"
            | "ol"
            | "p"
            | "pre"
            | "section"
            | "span"
            | "strong"
            | "sub"
            | "sup"
            | "table"
            | "tbody"
            | "td"
            | "th"
            | "thead"
            | "tr"
            | "ul"
            | "var"
    )
}

fn audit_declarations(
    declarations: &str,
    limits: ResourceLimits,
    violations: &mut Vec<StyleViolation>,
) {
    let policy = ResourcePolicy::new(limits);
    for declaration in declarations.split(';') {
        let declaration = declaration.trim();
        if declaration.is_empty() {
            continue;
        }
        let Some((property, value)) = declaration.split_once(':') else {
            violations.push(violation(declaration, "Malformed NODS declaration."));
            continue;
        };
        let property = property.trim().to_ascii_lowercase();
        let value = value.trim();
        if !is_allowed_property(&property) {
            violations.push(violation(&property, "Forbidden NODS property."));
        }
        audit_value(&property, value, policy, violations);
    }
}

fn audit_value(
    property: &str,
    value: &str,
    policy: ResourcePolicy,
    violations: &mut Vec<StyleViolation>,
) {
    let lower = value.to_ascii_lowercase();
    for forbidden in [
        "expression(",
        "attr(",
        "env(",
        "counter(",
        "counters(",
        "var(",
        "calc(",
        "min(",
        "max(",
        "clamp(",
    ] {
        if lower.contains(forbidden) {
            violations.push(violation(forbidden, "Forbidden NODS function."));
        }
    }
    for url in style_urls(value) {
        if policy.classify_uri(ReferenceKind::Style, url).is_err() {
            violations.push(violation(url, "Unsafe NODS URL."));
        }
    }
    if property == "position" && !matches!(lower.as_str(), "static" | "relative") {
        violations.push(violation(value, "Forbidden NODS positioning value."));
    }
    if property == "display"
        && !matches!(
            lower.as_str(),
            "block"
                | "inline"
                | "inline-block"
                | "list-item"
                | "table"
                | "table-row"
                | "table-cell"
                | "none"
        )
    {
        violations.push(violation(value, "Forbidden NODS display value."));
    }
}

fn is_allowed_property(property: &str) -> bool {
    matches!(
        property,
        "background"
            | "background-color"
            | "background-image"
            | "border"
            | "border-block"
            | "border-block-end"
            | "border-block-start"
            | "border-bottom"
            | "border-color"
            | "border-inline"
            | "border-inline-end"
            | "border-inline-start"
            | "border-left"
            | "border-radius"
            | "border-right"
            | "border-style"
            | "border-top"
            | "border-width"
            | "box-decoration-break"
            | "break-after"
            | "break-before"
            | "break-inside"
            | "color"
            | "display"
            | "font"
            | "font-family"
            | "font-size"
            | "font-style"
            | "font-weight"
            | "height"
            | "line-height"
            | "list-style"
            | "list-style-position"
            | "list-style-type"
            | "margin"
            | "margin-block"
            | "margin-block-end"
            | "margin-block-start"
            | "margin-bottom"
            | "margin-inline"
            | "margin-inline-end"
            | "margin-inline-start"
            | "margin-left"
            | "margin-right"
            | "margin-top"
            | "max-height"
            | "max-width"
            | "min-height"
            | "min-width"
            | "orphans"
            | "padding"
            | "padding-block"
            | "padding-block-end"
            | "padding-block-start"
            | "padding-bottom"
            | "padding-inline"
            | "padding-inline-end"
            | "padding-inline-start"
            | "padding-left"
            | "padding-right"
            | "padding-top"
            | "page-break-after"
            | "page-break-before"
            | "page-break-inside"
            | "position"
            | "size"
            | "text-align"
            | "text-decoration"
            | "text-transform"
            | "vertical-align"
            | "white-space"
            | "widows"
            | "width"
    )
}

fn escape_style_text(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '<' => out.push_str("\\3C "),
            '>' => out.push_str("\\3E "),
            _ => out.push(ch),
        }
    }
    out
}

fn dedupe(violations: &mut Vec<StyleViolation>) {
    let mut seen = BTreeSet::new();
    violations
        .retain(|violation| seen.insert((violation.construct.clone(), violation.message.clone())));
}

fn violation(construct: &str, message: &str) -> StyleViolation {
    StyleViolation {
        construct: construct.trim().to_string(),
        message: message.to_string(),
    }
}

fn is_ident(input: &str) -> bool {
    let mut chars = input.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_safe_nods_subset() {
        let source = "h1, .lead { color: #0f766e; font-size: 24pt; }\n@page { size: A4 portrait; margin: 22mm; }";
        assert!(audit_stylesheet(source, ResourceLimits::default()).is_safe());
        assert!(sanitize_stylesheet(source, ResourceLimits::default()).contains("@page"));
    }

    #[test]
    fn rejects_interactive_selectors_and_executable_functions() {
        let source = "a:hover { color: red; }\n.x { width: expression(alert(1)); }";
        let audit = audit_stylesheet(source, ResourceLimits::default());
        let constructs: Vec<_> = audit
            .violations
            .iter()
            .map(|violation| violation.construct.as_str())
            .collect();
        assert!(constructs.contains(&"a:hover"));
        assert!(constructs.contains(&"expression("));
    }

    #[test]
    fn validates_style_urls_through_policy() {
        let source = ".hero { background-image: url(https://example.test/a.png); }";
        let audit = audit_stylesheet(source, ResourceLimits::default());
        assert!(
            audit
                .violations
                .iter()
                .any(|v| v.message == "Unsafe NODS URL.")
        );
    }

    #[test]
    fn sanitizer_omits_unsafe_rules() {
        let source = "a:hover { color: red; }\np { color: blue; }";
        let sanitized = sanitize_stylesheet(source, ResourceLimits::default());
        assert!(!sanitized.contains(":hover"));
        assert!(sanitized.contains("color: blue"));
        assert!(sanitized.contains("NODX-E027"));
    }

    #[test]
    fn nods_security_corpus_is_rejected_or_accepted_deterministically() {
        let hostile =
            include_str!("../../../spec/tests/security/nods-hostile/forbidden-constructs.nodx");
        let safe = include_str!("../../../spec/tests/security/nods-hostile/safe-subset.nodx");
        assert!(
            audit_stylesheet(extract_style(hostile), ResourceLimits::default())
                .violations
                .len()
                >= 5
        );
        assert!(audit_stylesheet(extract_style(safe), ResourceLimits::default()).is_safe());
    }

    fn extract_style(input: &str) -> &str {
        input
            .split(":::style")
            .nth(1)
            .and_then(|rest| rest.split(":::").next())
            .expect("style fixture")
            .trim()
    }
}

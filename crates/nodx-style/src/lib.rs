#![forbid(unsafe_code)]

use std::collections::BTreeSet;

use nodx_url::{ReferenceKind, ResourceLimits, ResourcePolicy};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StyleViolation {
    pub construct: String,
    pub message: String,
    pub severity: Severity,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Severity {
    Error,
    Warning,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
        }
    }
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
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
        if audit_rule_to_vec(rule, limits)
            .iter()
            .all(|v| v.severity == Severity::Warning)
        {
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

pub fn yaml_style_to_css(input: &str) -> Result<String, String> {
    #[derive(Clone)]
    enum Context {
        Selector(String),
        Pseudo {
            key: String,
            selector: Option<String>,
        },
    }

    let mut out = String::new();
    let mut context: Option<Context> = None;
    for raw in input.lines() {
        if raw.trim().is_empty() || raw.trim_start().starts_with('#') {
            continue;
        }
        let indent = raw.chars().take_while(|ch| *ch == ' ').count();
        if indent != raw.len() - raw.trim_start().len() {
            return Err("YAML style indentation must use spaces.".to_string());
        }
        let line = raw.trim();
        if indent == 0 {
            let Some(name) = line.strip_suffix(':') else {
                return Err("YAML style top-level entries must end with `:`.".to_string());
            };
            context = Some(match name {
                "print" | "screen" | "dark" | "page" => Context::Pseudo {
                    key: name.to_string(),
                    selector: None,
                },
                selector => Context::Selector(selector.to_string()),
            });
            continue;
        }

        match &mut context {
            Some(Context::Selector(selector)) if indent >= 2 => {
                let (property, value) = split_yaml_declaration(line)?;
                push_css_rule(&mut out, selector, property, value);
            }
            Some(Context::Pseudo { key, selector }) if indent == 2 && line.ends_with(':') => {
                *selector = Some(line.trim_end_matches(':').to_string());
            }
            Some(Context::Pseudo {
                key,
                selector: Some(selector),
            }) if indent >= 4 => {
                let (property, value) = split_yaml_declaration(line)?;
                push_pseudo_rule(&mut out, key, selector, property, value);
            }
            Some(Context::Pseudo {
                key,
                selector: None,
            }) if *key == "page" && indent >= 2 => {
                let (property, value) = split_yaml_declaration(line)?;
                push_pseudo_rule(&mut out, key, "", property, value);
            }
            _ => return Err("Invalid YAML style structure.".to_string()),
        }
    }
    Ok(out)
}

fn split_yaml_declaration(line: &str) -> Result<(&str, &str), String> {
    let Some((property, value)) = line.split_once(':') else {
        return Err("YAML style declaration must use `property: value`.".to_string());
    };
    let property = property.trim();
    let value = value.trim().trim_matches('"');
    if property.is_empty() || value.is_empty() {
        return Err("YAML style declarations require a property and value.".to_string());
    }
    Ok((property, value))
}

fn push_css_rule(out: &mut String, selector: &str, property: &str, value: &str) {
    if !out.is_empty() {
        out.push('\n');
    }
    out.push_str(selector);
    out.push('{');
    out.push_str(property);
    out.push(':');
    out.push_str(value);
    out.push_str(";}");
}

fn push_pseudo_rule(out: &mut String, key: &str, selector: &str, property: &str, value: &str) {
    if !out.is_empty() {
        out.push('\n');
    }
    match key {
        "print" => out.push_str("@media print{"),
        "screen" => out.push_str("@media screen{"),
        "dark" => out.push_str("@media (prefers-color-scheme: dark){"),
        "page" => out.push_str("@page{"),
        _ => return,
    }
    if !selector.is_empty() {
        out.push_str(selector);
        out.push('{');
    }
    out.push_str(property);
    out.push(':');
    out.push_str(value);
    out.push(';');
    if !selector.is_empty() {
        out.push('}');
    }
    out.push('}');
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
        let inner = after[..end].trim();
        let raw = match (inner.starts_with('"') && inner.ends_with('"'))
            || (inner.starts_with('\'') && inner.ends_with('\''))
        {
            true => &inner[1..inner.len() - 1],
            false => inner,
        };
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
    let decoded = decode_css_escapes(input).to_ascii_lowercase();
    for construct in [
        "</style",
        "<script",
        "<svg",
        "<iframe",
        "<object",
        "<embed",
        "vbscript:",
        "expression(",
        "@import",
    ] {
        if decoded.contains(construct) {
            audit.violations.push(error(
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
        // Conditional at-rules wrap nested rules; recurse into them.
        let lower = prelude.to_ascii_lowercase();
        if lower.starts_with("@media") || lower.starts_with("@supports") {
            for nested in parse_rules(rule.declarations) {
                violations.extend(audit_rule_to_vec(nested, limits));
            }
            dedupe(&mut violations);
            return violations;
        }
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
    if lower.starts_with("@media") {
        return;
    }
    if lower.starts_with("@supports") {
        return;
    }
    violations.push(error(prelude, "Forbidden NODS at-rule."));
}

fn audit_selector(selector: &str, violations: &mut Vec<StyleViolation>) {
    if selector.is_empty() {
        violations.push(error("selector", "Missing NODS selector."));
        return;
    }
    for part in selector.split(',') {
        let part = part.trim();
        if part.is_empty() {
            violations.push(error("selector", "Empty NODS selector list entry."));
            continue;
        }
        for token in part.split_whitespace() {
            audit_selector_token(token, violations);
        }
    }
}

fn audit_selector_token(token: &str, violations: &mut Vec<StyleViolation>) {
    if token.is_empty() {
        violations.push(error("selector", "Empty NODS selector token."));
        return;
    }
    if matches!(token, ">" | "+" | "~") {
        // Combinators are accepted; child/sibling combinators are layout, not interactivity.
        return;
    }
    // Structural pseudo-classes (no user interaction, no resource loading)
    if matches!(
        token,
        ":root" | ":first-child" | ":last-child" | ":only-child" | ":empty"
    ) {
        return;
    }
    if let Some(arg) = token.strip_prefix(":not(")
        && let Some(inner) = arg.strip_suffix(')')
    {
        audit_selector_token(inner, violations);
        return;
    }
    if token.contains("::") {
        violations.push(error(token, "Pseudo-elements are not allowed in NODS."));
        return;
    }
    if token.contains(':') {
        violations.push(error(token, "Interactive NODS pseudo-class is forbidden."));
        return;
    }
    if token.contains('*') || token.contains('|') {
        violations.push(error(token, "Forbidden NODS selector."));
        return;
    }
    if let Some(start) = token.find('[') {
        let Some(end) = token.find(']') else {
            violations.push(error(token, "Unterminated attribute selector."));
            return;
        };
        if end <= start || token.len() != end + 1 + start.saturating_sub(start) {
            // not the only token, but we accept attribute selectors anchored to the end
        }
        let base = &token[..start];
        let attr = &token[start + 1..end];
        if !base.is_empty() && !is_safe_selector_base(base) {
            violations.push(error(base, "Unsupported NODS selector."));
            return;
        }
        if !is_safe_attribute_selector(attr) {
            violations.push(error(attr, "Forbidden NODS attribute selector."));
        }
        return;
    }
    if !is_safe_selector_base(token) {
        violations.push(error(token, "Unsupported NODS selector."));
    }
}

fn is_safe_selector_base(token: &str) -> bool {
    if token.is_empty() {
        return false;
    }
    // Support compound `tag.class.class2#id` selectors by recursing into the chain.
    if let Some(idx) = token.find(['.', '#']) {
        if idx == 0 {
            let kind = &token[..1];
            let rest = &token[1..];
            let next = rest.find(['.', '#']).unwrap_or(rest.len());
            let ident = &rest[..next];
            if !is_ident(ident) {
                return false;
            }
            if next == rest.len() {
                return kind == "." || kind == "#";
            }
            return is_safe_selector_base(&rest[next..]);
        }
        let base = &token[..idx];
        return is_safe_tag(base) && is_safe_selector_base(&token[idx..]);
    }
    is_safe_tag(token)
}

fn is_safe_tag(token: &str) -> bool {
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
            | "main"
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

fn is_safe_attribute_selector(attr: &str) -> bool {
    // accept [name], [name=value], [name="value"], [name~="value"], [name|="value"], [name^="value"], [name$="value"], [name*="value"]
    let mut chars = attr.chars();
    let mut name = String::new();
    while let Some(c) = chars.clone().next() {
        if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
            name.push(c);
            chars.next();
        } else {
            break;
        }
    }
    if name.is_empty() || !is_ident(&name) {
        return false;
    }
    let rest: String = chars.collect();
    if rest.is_empty() {
        return matches!(
            name.as_str(),
            "lang"
                | "dir"
                | "role"
                | "data-tag"
                | "data-tone"
                | "data-variant"
                | "data-color"
                | "data-status"
        );
    }
    let rest = rest.trim();
    let operators = ["~=", "|=", "^=", "$=", "*=", "="];
    for op in &operators {
        if let Some(value) = rest.strip_prefix(op) {
            let value = value.trim();
            if (value.starts_with('"') && value.ends_with('"') && value.len() >= 2)
                || (value.starts_with('\'') && value.ends_with('\'') && value.len() >= 2)
            {
                let inner = &value[1..value.len() - 1];
                return !inner.is_empty() && !inner.contains(['<', '>', '"', '\'']);
            }
            return false;
        }
    }
    false
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
            violations.push(error(declaration, "Malformed NODS declaration."));
            continue;
        };
        let property = property.trim().to_ascii_lowercase();
        let value = value.trim();
        if is_forbidden_property(&property) {
            violations.push(error(&property, "Forbidden NODS property."));
            continue;
        }
        if !is_allowed_property(&property) {
            violations.push(warning(&property, "Unsupported NODS property."));
            continue;
        }
        audit_value(&property, value, policy, violations);
    }
}

fn is_forbidden_property(property: &str) -> bool {
    matches!(
        property,
        "behavior" | "-moz-binding" | "-ms-behavior" | "binding"
    )
}

fn audit_value(
    property: &str,
    value: &str,
    policy: ResourcePolicy,
    violations: &mut Vec<StyleViolation>,
) {
    let decoded = decode_css_escapes(value);
    let lower = decoded.to_ascii_lowercase();
    for forbidden in [
        "expression(",
        "javascript:",
        "vbscript:",
        "@import",
        "behavior:",
    ] {
        if lower.contains(forbidden) {
            violations.push(error(forbidden, "Forbidden NODS construct in value."));
        }
    }
    for url in style_urls(value) {
        if policy.classify_uri(ReferenceKind::Style, url).is_err() {
            violations.push(error(url, "Unsafe NODS URL."));
        }
    }
    if property == "position" && !matches!(lower.as_str(), "static" | "relative") {
        violations.push(error(value, "Forbidden NODS positioning value."));
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
                | "table-header-group"
                | "table-row-group"
                | "table-footer-group"
                | "none"
                | "flex"
                | "inline-flex"
                | "grid"
                | "inline-grid"
        )
    {
        violations.push(error(value, "Forbidden NODS display value."));
    }
}

fn is_allowed_property(property: &str) -> bool {
    if property.starts_with("--") && property.len() > 2 {
        return true; // CSS custom properties / design tokens are inert content
    }
    matches!(
        property,
        "background"
            | "background-color"
            | "background-image"
            | "background-position"
            | "background-repeat"
            | "background-size"
            | "border"
            | "border-block"
            | "border-block-end"
            | "border-block-start"
            | "border-bottom"
            | "border-collapse"
            | "border-color"
            | "border-inline"
            | "border-inline-end"
            | "border-inline-start"
            | "border-left"
            | "border-radius"
            | "border-right"
            | "border-spacing"
            | "border-style"
            | "border-top"
            | "border-width"
            | "box-decoration-break"
            | "box-shadow"
            | "break-after"
            | "break-before"
            | "break-inside"
            | "color"
            | "column-count"
            | "column-gap"
            | "column-width"
            | "columns"
            | "direction"
            | "display"
            | "flex"
            | "flex-basis"
            | "flex-direction"
            | "flex-grow"
            | "flex-shrink"
            | "flex-wrap"
            | "font"
            | "font-family"
            | "font-feature-settings"
            | "font-size"
            | "font-style"
            | "font-variant"
            | "font-weight"
            | "gap"
            | "grid-column"
            | "grid-column-gap"
            | "grid-row"
            | "grid-row-gap"
            | "grid-template-columns"
            | "grid-template-rows"
            | "hanging-punctuation"
            | "height"
            | "hyphens"
            | "justify-content"
            | "justify-items"
            | "letter-spacing"
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
            | "opacity"
            | "orphans"
            | "outline"
            | "outline-color"
            | "outline-offset"
            | "outline-style"
            | "outline-width"
            | "overflow"
            | "overflow-wrap"
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
            | "quotes"
            | "size"
            | "tab-size"
            | "table-layout"
            | "text-align"
            | "text-decoration"
            | "text-decoration-color"
            | "text-decoration-style"
            | "text-decoration-thickness"
            | "text-indent"
            | "text-transform"
            | "text-underline-offset"
            | "vertical-align"
            | "white-space"
            | "widows"
            | "width"
            | "word-break"
            | "word-spacing"
            | "writing-mode"
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

fn decode_css_escapes(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            // Up to 6 hex digits, optional trailing whitespace
            let mut j = i + 1;
            let mut hex = String::new();
            while j < bytes.len() && hex.len() < 6 && (bytes[j] as char).is_ascii_hexdigit() {
                hex.push(bytes[j] as char);
                j += 1;
            }
            if !hex.is_empty() {
                if j < bytes.len() && (bytes[j] == b' ' || bytes[j] == b'\t' || bytes[j] == b'\n') {
                    j += 1;
                }
                if let Ok(code) = u32::from_str_radix(&hex, 16)
                    && let Some(ch) = char::from_u32(code)
                {
                    out.push(ch);
                }
                i = j;
                continue;
            }
            if j < bytes.len() {
                out.push(bytes[j] as char);
                i = j + 1;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn dedupe(violations: &mut Vec<StyleViolation>) {
    let mut seen = BTreeSet::new();
    violations
        .retain(|v| seen.insert((v.construct.clone(), v.message.clone(), v.severity.as_str())));
}

fn error(construct: &str, message: &str) -> StyleViolation {
    StyleViolation {
        construct: construct.trim().to_string(),
        message: message.to_string(),
        severity: Severity::Error,
    }
}

fn warning(construct: &str, message: &str) -> StyleViolation {
    StyleViolation {
        construct: construct.trim().to_string(),
        message: message.to_string(),
        severity: Severity::Warning,
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
        let source = "h1, .lead { color: #0f766e; font-size: 24pt; }\n@page { size: A4 portrait; margin: 22mm; }\n[lang=\"ar\"] { text-align: right; }\n@media print { p { color: black; } }";
        let audit = audit_stylesheet(source, ResourceLimits::default());
        assert!(
            audit.is_safe(),
            "expected safe but got: {:?}",
            audit.violations
        );
    }

    #[test]
    fn rejects_interactive_selectors_and_executable_functions() {
        let source = "a:hover { color: red; }\n.x { width: expression(alert(1)); }";
        let audit = audit_stylesheet(source, ResourceLimits::default());
        assert!(
            audit
                .violations
                .iter()
                .any(|v| v.severity == Severity::Error)
        );
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
    fn allows_calc_and_var_and_clamp() {
        let source = ".x { padding: clamp(8px, calc(1vw + 4px), 32px); width: max(50%, 200px); --tone: blue; }";
        let audit = audit_stylesheet(source, ResourceLimits::default());
        // No `error` severity should be present for the value side
        assert!(
            audit
                .violations
                .iter()
                .all(|v| v.severity != Severity::Error),
            "violations: {:?}",
            audit.violations
        );
    }

    #[test]
    fn rejects_css_escape_breakouts() {
        let source = ".x { content: '\\3c script\\3e '; }";
        let audit = audit_stylesheet(source, ResourceLimits::default());
        assert!(
            audit
                .violations
                .iter()
                .any(|v| v.message == "Forbidden executable or breakout content in NODS.")
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
        let hostile_audit = audit_stylesheet(extract_style(hostile), ResourceLimits::default());
        assert!(
            hostile_audit
                .violations
                .iter()
                .filter(|v| v.severity == Severity::Error)
                .count()
                >= 5
        );
        assert!(audit_stylesheet(extract_style(safe), ResourceLimits::default()).is_safe());
    }

    #[test]
    fn nods_hostile_corpus_scan_emits_errors() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let dir = root.join("spec/tests/security/nods-hostile");
        let mut count = 0;
        for entry in std::fs::read_dir(&dir).expect("nods-hostile dir") {
            let entry = entry.expect("dirent");
            let path = entry.path();
            let name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n,
                None => continue,
            };
            if !name.ends_with(".nodx") {
                continue;
            }
            if name == "safe-subset.nodx" {
                continue;
            }
            let source = std::fs::read_to_string(&path).expect("read fixture");
            let style = extract_style(&source);
            let audit = audit_stylesheet(style, ResourceLimits::default());
            assert!(
                audit
                    .violations
                    .iter()
                    .any(|v| v.severity == Severity::Error),
                "expected at least one error severity in {name}, got {:?}",
                audit.violations
            );
            count += 1;
        }
        assert!(
            count >= 28,
            "expected at least 28 hostile NODS fixtures, found {count}"
        );
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

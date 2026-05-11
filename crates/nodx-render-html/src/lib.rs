#![forbid(unsafe_code)]

use nodx_core::{
    Document, Inline, NavigationGraph, Node, ResourceLimits, Value, default_navigation_label,
    resolve_navigation, sha256_bytes,
};
use nodx_style::sanitize_stylesheet;
use nodx_url::{ReferenceKind, ResourcePolicy};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RenderOptions {
    pub standalone: bool,
    pub include_csp: bool,
    pub limits: ResourceLimits,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            standalone: true,
            include_csp: true,
            limits: ResourceLimits::default(),
        }
    }
}

pub fn render_html(doc: &Document) -> String {
    render_html_with_options(doc, RenderOptions::default())
}

pub fn render_html_with_limits(doc: &Document, limits: ResourceLimits) -> String {
    render_html_with_options(
        doc,
        RenderOptions {
            limits,
            ..RenderOptions::default()
        },
    )
}

pub fn render_fragment(doc: &Document) -> String {
    render_html_with_options(
        doc,
        RenderOptions {
            standalone: false,
            include_csp: false,
            ..RenderOptions::default()
        },
    )
}

pub fn render_html_with_options(doc: &Document, options: RenderOptions) -> String {
    let navigation = resolve_navigation(doc);
    let policy = ResourcePolicy::new(options.limits);
    let body = render_body(doc, &navigation, policy);
    if !options.standalone {
        return body;
    }

    let stylesheet = base_stylesheet(doc);
    let style_hash = sha256_base64_for_csp(stylesheet.as_bytes());

    let lang = match doc.meta.get("language") {
        Some(Value::String(s)) if s != "und" => s.clone(),
        _ => String::new(),
    };
    let dir = match doc.meta.get("dir") {
        Some(Value::String(s)) if s != "auto" => s.clone(),
        _ => String::new(),
    };
    let mut html_attrs = String::new();
    if !lang.is_empty() {
        html_attrs.push_str(" lang=\"");
        escape_attr(&mut html_attrs, &lang);
        html_attrs.push('"');
    }
    if !dir.is_empty() {
        html_attrs.push_str(" dir=\"");
        escape_attr(&mut html_attrs, &dir);
        html_attrs.push('"');
    }

    let mut out = String::new();
    out.push_str("<!doctype html><html");
    out.push_str(&html_attrs);
    out.push_str("><meta charset=\"utf-8\">");
    if options.include_csp {
        out.push_str("<meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; img-src 'self' data:; style-src 'sha256-");
        out.push_str(&style_hash);
        out.push_str("'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'\">");
    }
    out.push_str("<style>");
    out.push_str(&stylesheet);
    out.push_str("</style>");

    let title = match doc.meta.get("title") {
        Some(Value::String(s)) => Some(s.clone()),
        _ => derive_title(&doc.body),
    };
    if let Some(title) = title {
        out.push_str("<title>");
        escape_html(&mut out, &title);
        out.push_str("</title>");
    }
    out.push_str(&body);
    out.push_str("</html>");
    out
}

fn render_body(doc: &Document, navigation: &NavigationGraph, policy: ResourcePolicy) -> String {
    let mut out = String::new();
    for (i, node) in doc.body.iter().enumerate() {
        render_node(&mut out, node, &i.to_string(), navigation, policy);
    }
    out
}

fn derive_title(nodes: &[Node]) -> Option<String> {
    for node in nodes {
        if node.node_type == "heading" {
            let mut out = String::new();
            for inline in &node.inlines {
                if let Inline::Text(t) = inline {
                    out.push_str(t);
                }
            }
            if !out.trim().is_empty() {
                return Some(out);
            }
        }
        if let Some(t) = derive_title(&node.children) {
            return Some(t);
        }
    }
    None
}

fn base_stylesheet(_doc: &Document) -> String {
    // Static base stylesheet — fixed bytes so its CSP sha256 hash is deterministic
    String::from("html{font-family:system-ui}html[dir=\"rtl\"]{direction:rtl}body{font:16px/1.6 system-ui;max-width:920px;margin:32px auto;padding:0 16px;color:#1f2937}h1,h2,h3,h4,h5,h6{line-height:1.25;color:#0f172a;margin-top:1.4em}p{margin:0 0 1em}pre{padding:12px;background:#f5f5f5;overflow:auto;border-radius:6px}code{font-family:ui-monospace,SFMono-Regular,Menlo,monospace}aside{border-inline-start:4px solid #b57f00;padding:8px 12px;background:#fff8e6}table{border-collapse:collapse;margin:0 0 1em}td,th{border:1px solid #d1d5db;padding:6px 10px}thead th{background:#f3f4f6;text-align:start}figure{margin:1.5em 0}figcaption{font-size:0.9em;color:#4b5563}nav ol{padding-inline-start:1.5rem}nav strong{display:block;margin-bottom:0.4em}.nodx-blocked-link,.nodx-blocked-image{color:#b91c1c;text-decoration:line-through}.nodx-blocked-link{cursor:not-allowed}.mention{font-variant:all-small-caps}.pagebreak{border:none;border-top:1px dashed #9ca3af;margin:2em 0}.math-inline{background:#f3f4f6;padding:1px 4px;border-radius:3px}")
}

fn sha256_base64_for_csp(input: &[u8]) -> String {
    let digest = sha256_bytes(input);
    // CSP hash uses standard base64 with padding, not base64url
    base64_standard(&digest)
}

fn base64_standard(input: &[u8]) -> String {
    const ALPHABET: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    let mut i = 0;
    while i + 3 <= input.len() {
        let n = ((input[i] as u32) << 16) | ((input[i + 1] as u32) << 8) | input[i + 2] as u32;
        out.push(ALPHABET[((n >> 18) & 63) as usize] as char);
        out.push(ALPHABET[((n >> 12) & 63) as usize] as char);
        out.push(ALPHABET[((n >> 6) & 63) as usize] as char);
        out.push(ALPHABET[(n & 63) as usize] as char);
        i += 3;
    }
    match input.len() - i {
        1 => {
            let n = (input[i] as u32) << 16;
            out.push(ALPHABET[((n >> 18) & 63) as usize] as char);
            out.push(ALPHABET[((n >> 12) & 63) as usize] as char);
            out.push('=');
            out.push('=');
        }
        2 => {
            let n = ((input[i] as u32) << 16) | ((input[i + 1] as u32) << 8);
            out.push(ALPHABET[((n >> 18) & 63) as usize] as char);
            out.push(ALPHABET[((n >> 12) & 63) as usize] as char);
            out.push(ALPHABET[((n >> 6) & 63) as usize] as char);
            out.push('=');
        }
        _ => {}
    }
    out
}

fn render_node(
    out: &mut String,
    node: &Node,
    path: &str,
    navigation: &NavigationGraph,
    policy: ResourcePolicy,
) {
    match node.node_type.as_str() {
        "heading" => {
            let level = node
                .attrs
                .get("level")
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(1)
                .clamp(1, 6);
            out.push_str(&format!("<h{}{}>", level, html_id(node)));
            render_inlines(out, &node.inlines, policy);
            out.push_str(&format!("</h{}>", level));
        }
        "paragraph" => wrap_inlines(out, "p", node, path, navigation, policy),
        "section" => wrap_children(out, "section", node, path, navigation, policy),
        "note" => wrap_children(out, "aside", node, path, navigation, policy),
        "quote" => wrap_children(out, "blockquote", node, path, navigation, policy),
        "list" => {
            let tag = if node.attrs.get("kind").map(|s| s.as_str()) == Some("ordered") {
                "ol"
            } else {
                "ul"
            };
            out.push_str(tag_open(tag, node).as_str());
            for (i, child) in node.children.iter().enumerate() {
                render_node(out, child, &child_path(path, i), navigation, policy);
            }
            out.push_str(&format!("</{}>", tag));
        }
        "item" => wrap_inlines(out, "li", node, path, navigation, policy),
        "code" | "pre" => {
            out.push_str("<pre><code");
            if let Some(lang) = node.attrs.get("lang") {
                out.push_str(" data-lang=\"");
                escape_attr(out, lang);
                out.push('"');
            }
            out.push('>');
            escape_html(out, node.text.as_deref().unwrap_or(""));
            out.push_str("</code></pre>");
        }
        "math" => {
            out.push_str("<pre class=\"math\">");
            escape_html(out, node.text.as_deref().unwrap_or(""));
            out.push_str("</pre>");
        }
        "style" => {
            out.push_str("<style>");
            out.push_str(&sanitize_stylesheet(
                node.text.as_deref().unwrap_or(""),
                policy.limits(),
            ));
            out.push_str("</style>");
        }
        "table" => wrap_children(out, "table", node, path, navigation, policy),
        "row" => wrap_children(out, "tr", node, path, navigation, policy),
        "cell" => {
            let tag = if node.attrs.get("header").map(|s| s.as_str()) == Some("true") {
                "th"
            } else {
                "td"
            };
            wrap_inlines(out, tag, node, path, navigation, policy);
        }
        "figure" => wrap_children(out, "figure", node, path, navigation, policy),
        "caption" => wrap_inlines(out, "figcaption", node, path, navigation, policy),
        "image" => {
            let alt = node.attrs.get("alt").map(String::as_str).unwrap_or("");
            let safe_src = node
                .attrs
                .get("src")
                .and_then(|s| safe_image_url_with_policy(policy, s));
            match safe_src {
                Some(src) => {
                    out.push_str("<img src=\"");
                    escape_attr(out, &src);
                    out.push_str("\" alt=\"");
                    escape_attr(out, alt);
                    out.push_str("\">");
                }
                None => {
                    out.push_str("<span class=\"nodx-blocked-image\">");
                    escape_html(out, if alt.is_empty() { "blocked image" } else { alt });
                    out.push_str("</span>");
                }
            }
        }
        "form" => wrap_children(out, "dl", node, path, navigation, policy),
        "field" => {
            out.push_str("<div");
            out.push_str(&html_id(node));
            out.push_str("><dt>");
            escape_html(
                out,
                node.attrs
                    .get("label")
                    .or_else(|| node.attrs.get("name"))
                    .map(String::as_str)
                    .unwrap_or("Field"),
            );
            out.push_str("</dt><dd>");
            escape_html(
                out,
                node.attrs.get("value").map(String::as_str).unwrap_or(""),
            );
            out.push_str("</dd></div>");
        }
        "toc" => {
            out.push_str("<nav");
            out.push_str(&html_id(node));
            let nav = navigation
                .navigations
                .iter()
                .find(|candidate| candidate.toc_path == path);
            let label = nav.map(|nav| nav.label.as_str()).unwrap_or_else(|| {
                node.attrs
                    .get("role")
                    .map(|role| default_navigation_label(role))
                    .unwrap_or("Table of contents")
            });
            out.push_str(" aria-label=\"");
            escape_attr(out, label);
            out.push_str("\"><strong>");
            escape_html(out, label);
            out.push_str("</strong>");
            if let Some(nav) = nav {
                if nav.entries.is_empty() {
                    out.push_str("<p class=\"nodx-toc-empty\">");
                    escape_html(out, "(no entries)");
                    out.push_str("</p>");
                } else {
                    out.push_str("<ol>");
                    for entry in &nav.entries {
                        if !is_safe_fragment_id(&entry.id) {
                            continue;
                        }
                        out.push_str("<li><a href=\"#");
                        escape_attr(out, &entry.id);
                        out.push_str("\">");
                        escape_html(out, &entry.title);
                        out.push_str("</a></li>");
                    }
                    out.push_str("</ol>");
                }
            } else {
                out.push_str("<p class=\"nodx-toc-empty\">");
                escape_html(out, "(no entries)");
                out.push_str("</p>");
            }
            out.push_str("</nav>");
        }
        "pagebreak" => {
            out.push_str("<hr");
            out.push_str(&html_id(node));
            out.push_str(" class=\"pagebreak\">");
        }
        "media" | "embed" => {
            out.push_str("<figure");
            out.push_str(&html_id(node));
            out.push_str("><div class=\"media-fallback\">");
            escape_html(
                out,
                node.attrs
                    .get("alt")
                    .map(String::as_str)
                    .unwrap_or(&node.node_type),
            );
            if let Some(src) = node.attrs.get("src")
                && (ResourcePolicy::new(policy.limits())
                    .classify_uri(ReferenceKind::MediaFallback, src)
                    .is_ok()
                    || ResourcePolicy::new(policy.limits())
                        .classify_uri(ReferenceKind::Asset, src)
                        .is_ok())
            {
                out.push_str(" - ");
                escape_html(out, src);
            }
            out.push_str("</div>");
            for (i, child) in node.children.iter().enumerate() {
                render_node(out, child, &child_path(path, i), navigation, policy);
            }
            out.push_str("</figure>");
        }
        "bibliography" => wrap_children(out, "ol", node, path, navigation, policy),
        "citation-entry" => wrap_inlines(out, "li", node, path, navigation, policy),
        "speaker-notes" => {
            out.push_str("<aside");
            out.push_str(&html_id(node));
            out.push_str(" class=\"speaker-notes\" aria-label=\"Speaker notes\">");
            render_inlines(out, &node.inlines, policy);
            for (i, child) in node.children.iter().enumerate() {
                render_node(out, child, &child_path(path, i), navigation, policy);
            }
            out.push_str("</aside>");
        }
        _ => wrap_children(out, "div", node, path, navigation, policy),
    }
}

fn wrap_children(
    out: &mut String,
    tag: &str,
    node: &Node,
    path: &str,
    navigation: &NavigationGraph,
    policy: ResourcePolicy,
) {
    out.push_str(tag_open(tag, node).as_str());
    for (i, child) in node.children.iter().enumerate() {
        render_node(out, child, &child_path(path, i), navigation, policy);
    }
    out.push_str(&format!("</{}>", tag));
}

fn wrap_inlines(
    out: &mut String,
    tag: &str,
    node: &Node,
    path: &str,
    navigation: &NavigationGraph,
    policy: ResourcePolicy,
) {
    out.push_str(tag_open(tag, node).as_str());
    render_inlines(out, &node.inlines, policy);
    for (i, child) in node.children.iter().enumerate() {
        render_node(out, child, &child_path(path, i), navigation, policy);
    }
    out.push_str(&format!("</{}>", tag));
}

fn child_path(prefix: &str, index: usize) -> String {
    if prefix.is_empty() {
        index.to_string()
    } else {
        format!("{prefix}.{index}")
    }
}

fn tag_open(tag: &str, node: &Node) -> String {
    format!("<{}{}>", tag, html_attrs(node))
}

fn html_id(node: &Node) -> String {
    match &node.id {
        Some(id) => {
            let mut s = String::from(" id=\"");
            escape_attr(&mut s, id);
            s.push('"');
            s
        }
        None => String::new(),
    }
}

fn html_attrs(node: &Node) -> String {
    let mut s = html_id(node);
    if !node.classes.is_empty() {
        s.push_str(" class=\"");
        for (i, c) in node.classes.iter().enumerate() {
            if i > 0 {
                s.push(' ');
            }
            escape_attr(&mut s, c);
        }
        s.push('"');
    }
    if let Some(lang) = node.attrs.get("lang") {
        s.push_str(" lang=\"");
        escape_attr(&mut s, lang);
        s.push('"');
    }
    if let Some(dir) = node.attrs.get("dir")
        && matches!(dir.as_str(), "ltr" | "rtl" | "auto")
    {
        s.push_str(" dir=\"");
        escape_attr(&mut s, dir);
        s.push('"');
    }
    if let Some(title) = node.attrs.get("title") {
        s.push_str(" title=\"");
        escape_attr(&mut s, title);
        s.push('"');
    }
    s
}

fn render_inlines(out: &mut String, inlines: &[Inline], policy: ResourcePolicy) {
    for item in inlines {
        match item {
            Inline::Text(text) => escape_html(out, text),
            Inline::Strong(children) => {
                out.push_str("<strong>");
                render_inlines(out, children, policy);
                out.push_str("</strong>");
            }
            Inline::Em(children) => {
                out.push_str("<em>");
                render_inlines(out, children, policy);
                out.push_str("</em>");
            }
            Inline::Code(text) => {
                out.push_str("<code>");
                escape_html(out, text);
                out.push_str("</code>");
            }
            Inline::Link { label, target } => match safe_link_url_with_policy(policy, target) {
                Some(safe) => {
                    out.push_str("<a href=\"");
                    escape_attr(out, &safe);
                    out.push_str("\" rel=\"noopener noreferrer\">");
                    render_inlines(out, label, policy);
                    out.push_str("</a>");
                }
                None => {
                    out.push_str("<a class=\"nodx-blocked-link\" data-blocked=\"");
                    escape_attr(out, target);
                    out.push_str("\" title=\"Blocked unsafe URL\">");
                    render_inlines(out, label, policy);
                    out.push_str("</a>");
                }
            },
            Inline::Span { children, attrs } => {
                out.push_str("<span");
                if !attrs.classes.is_empty() {
                    out.push_str(" class=\"");
                    for (i, class) in attrs.classes.iter().enumerate() {
                        if i > 0 {
                            out.push(' ');
                        }
                        escape_attr(out, class);
                    }
                    out.push('"');
                }
                if let Some(id) = &attrs.id {
                    out.push_str(" id=\"");
                    escape_attr(out, id);
                    out.push('"');
                }
                if let Some(lang) = attrs.attrs.get("lang") {
                    out.push_str(" lang=\"");
                    escape_attr(out, lang);
                    out.push('"');
                }
                if let Some(dir) = attrs.attrs.get("dir")
                    && matches!(dir.as_str(), "ltr" | "rtl" | "auto")
                {
                    out.push_str(" dir=\"");
                    escape_attr(out, dir);
                    out.push('"');
                }
                if let Some(title) = attrs.attrs.get("title") {
                    out.push_str(" title=\"");
                    escape_attr(out, title);
                    out.push('"');
                }
                out.push('>');
                render_inlines(out, children, policy);
                out.push_str("</span>");
            }
            Inline::Mark(children) => {
                out.push_str("<mark>");
                render_inlines(out, children, policy);
                out.push_str("</mark>");
            }
            Inline::Sub(children) => {
                out.push_str("<sub>");
                render_inlines(out, children, policy);
                out.push_str("</sub>");
            }
            Inline::Sup(children) => {
                out.push_str("<sup>");
                render_inlines(out, children, policy);
                out.push_str("</sup>");
            }
            Inline::Var { namespace, name } => {
                out.push_str("<var>");
                escape_html(out, namespace);
                out.push('.');
                escape_html(out, name);
                out.push_str("</var>");
            }
            Inline::Ref { target } => {
                if is_safe_fragment_id(target) {
                    out.push_str("<a href=\"#");
                    escape_attr(out, target);
                    out.push_str("\">@");
                    escape_html(out, target);
                    out.push_str("</a>");
                } else {
                    out.push_str("<span class=\"nodx-blocked-link\">@");
                    escape_html(out, target);
                    out.push_str("</span>");
                }
            }
            Inline::Mention { kind, target } => {
                out.push_str("<span class=\"mention\">@");
                escape_html(out, kind);
                out.push(':');
                escape_html(out, target);
                out.push_str("</span>");
            }
            Inline::FootnoteRef { target } | Inline::CitationRef { target } => {
                if is_safe_fragment_id(target) {
                    out.push_str("<a href=\"#");
                    escape_attr(out, target);
                    out.push_str("\">[");
                    escape_html(out, target);
                    out.push_str("]</a>");
                } else {
                    out.push_str("<span class=\"nodx-blocked-link\">[");
                    escape_html(out, target);
                    out.push_str("]</span>");
                }
            }
            Inline::MathInline { source } => {
                out.push_str("<code class=\"math-inline\">");
                escape_html(out, source);
                out.push_str("</code>");
            }
        }
    }
}

fn escape_html(out: &mut String, input: &str) {
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            // Strip XML/HTML-invalid control chars (allow tab/LF/CR)
            c if (c as u32) < 0x20 && c != '\t' && c != '\n' && c != '\r' => {}
            _ => out.push(ch),
        }
    }
}

pub fn safe_link_url(raw: &str) -> Option<String> {
    safe_link_url_with_policy(ResourcePolicy::default(), raw)
}

pub fn safe_image_url(raw: &str) -> Option<String> {
    safe_image_url_with_policy(ResourcePolicy::default(), raw)
}

pub fn is_safe_asset_ref(raw: &str) -> bool {
    ResourcePolicy::default()
        .classify_uri(ReferenceKind::Asset, raw)
        .is_ok()
}

fn safe_link_url_with_policy(policy: ResourcePolicy, raw: &str) -> Option<String> {
    policy
        .classify_uri(ReferenceKind::Link, raw)
        .ok()
        .map(|uri| uri.raw)
}

fn safe_image_url_with_policy(policy: ResourcePolicy, raw: &str) -> Option<String> {
    policy
        .classify_uri(ReferenceKind::Asset, raw)
        .ok()
        .map(|uri| uri.raw)
}

fn escape_attr(out: &mut String, input: &str) {
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            c if (c as u32) < 0x20 && c != '\t' && c != '\n' && c != '\r' => {}
            _ => out.push(ch),
        }
    }
}

fn is_safe_fragment_id(input: &str) -> bool {
    let stripped = input.strip_prefix('#').unwrap_or(input);
    let mut chars = stripped.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

#[cfg(test)]
mod tests {
    use nodx_core::parse_str;

    use super::*;

    #[test]
    fn html_render_blocks_javascript_link() {
        let doc = parse_str("[click](java\u{73}cript:alert(1))\n");
        let html = render_html(&doc);
        assert!(!html.contains("href=\"javascript"));
        assert!(html.contains("nodx-blocked-link"));
    }

    #[test]
    fn standalone_html_emits_strict_csp_with_style_hash() {
        let doc = parse_str("# T\n");
        let html = render_html(&doc);
        assert!(html.contains("Content-Security-Policy"));
        assert!(html.contains("default-src 'none'"));
        assert!(html.contains("style-src 'sha256-"));
        assert!(!html.contains("'unsafe-inline'"));
        assert!(!html.to_ascii_lowercase().contains("<script"));
    }

    #[test]
    fn fragment_render_omits_csp_and_doctype() {
        let doc = parse_str("# T\n");
        let html = render_fragment(&doc);
        assert!(!html.contains("Content-Security-Policy"));
        assert!(!html.contains("<!doctype"));
        assert!(html.contains("<h1>T</h1>"));
    }

    #[test]
    fn style_block_is_literal_and_emits_style_tag() {
        let doc = parse_str(":::style\nh1 { color: red; }\n:::\n");
        let html = render_html(&doc);
        assert!(html.contains("<style>h1 { color: red; }</style>"));
    }

    #[test]
    fn style_block_blocks_html_breakout() {
        let doc =
            parse_str(":::style\nbody { color: red; } </style><script>alert(1)</script>\n:::\n");
        let html = render_html(&doc);
        assert!(!html.to_lowercase().contains("<script"));
        assert!(html.contains("NODX-E027"));
    }

    #[test]
    fn forbidden_nods_emits_inline_omit_marker() {
        let doc = parse_str(
            ":::style\na:hover { color: red; }\np { color: blue; }\n:::\n",
        );
        let html = render_html(&doc);
        assert!(!html.contains(":hover"));
        assert!(html.contains("color: blue"));
        assert!(html.contains("forbidden NODS rule omitted"));
    }

    #[test]
    fn style_url_policy_blocks_remote_urls() {
        let doc = parse_str(
            ":::style\n.hero { background-image: url(https://example.test/a.png); }\n:::\n",
        );
        let html = render_html(&doc);
        assert!(!html.contains("https://example.test/a.png"));
    }

    #[test]
    fn html_emits_lang_and_dir_on_root() {
        let doc = parse_str("---\nschema: nodx/1.0\nlanguage: ar\ndir: rtl\n---\n\n# T\n");
        let html = render_html(&doc);
        assert!(html.contains("<html lang=\"ar\" dir=\"rtl\">"));
    }

    #[test]
    fn inline_i18n_attrs_render_to_html() {
        let doc = parse_str("[٩٨ ريال]{lang=\"ar\" dir=\"rtl\" title=\"price\"}\n");
        let html = render_html(&doc);
        assert!(html.contains("<span lang=\"ar\" dir=\"rtl\" title=\"price\">"));
    }

    #[test]
    fn toc_renders_deterministic_navigation_links() {
        let doc = parse_str(":::toc {role=\"local\"}\n:::\n\n# A {#a}\n\n## B {#b}\n");
        let html = render_html(&doc);
        assert!(html.contains("<nav aria-label=\"In this section\"><strong>In this section</strong><ol><li><a href=\"#a\">A</a></li><li><a href=\"#b\">B</a></li></ol></nav>"));
    }

    #[test]
    fn xss_corpus_is_escaped_or_blocked() {
        let doc = parse_str(include_str!(
            "../../../spec/tests/security/xss/html-contexts.nodx"
        ));
        let html = render_html(&doc);
        assert!(!html.to_ascii_lowercase().contains("<script"));
        assert!(!html.to_ascii_lowercase().contains("<img src=x onerror"));
        assert!(!html.contains("href=\"javascript"));
        assert!(html.contains("&lt;img src=x"));
    }

    #[test]
    fn xss_corpus_directory_scan_is_neutralized() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let dir = root.join("spec/tests/security/xss");
        let mut count = 0;
        for entry in std::fs::read_dir(&dir).expect("xss dir") {
            let entry = entry.expect("dirent");
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("nodx") {
                continue;
            }
            let source = std::fs::read_to_string(&path).expect("read fixture");
            let doc = parse_str(&source);
            let html = render_html(&doc).to_ascii_lowercase();
            for tag in [
                "<script", "<iframe", "<object", "<embed", "<svg",
            ] {
                assert!(
                    !html.contains(tag),
                    "{:?} leaked {tag}",
                    path.file_name()
                );
            }
            for href in [
                "href=\"javascript",
                "href=\"vbscript",
                "src=\"javascript",
                "src=\"vbscript",
                "src=\"data:text/html",
                "src=\"data:image/svg",
            ] {
                assert!(
                    !html.contains(href),
                    "{:?} leaked {href}",
                    path.file_name()
                );
            }
            count += 1;
        }
        assert!(count >= 40, "expected at least 40 XSS fixtures, found {count}");
    }

    #[test]
    fn xss_payload_corpus_is_neutralized() {
        let doc =
            parse_str(include_str!("../../../spec/tests/security/xss/payloads.nodx"));
        let html = render_html(&doc).to_ascii_lowercase();
        // No executable HTML elements
        for tag in [
            "<script", "<iframe", "<object", "<embed", "<svg", "<img src=x",
        ] {
            assert!(!html.contains(tag), "renderer leaked tag `{tag}`");
        }
        // No active hrefs to unsafe schemes
        for href in [
            "href=\"javascript",
            "href=\"vbscript",
            "href=\"data:text/html",
            "href=\"data:application/xhtml",
            "href=\"data:image/svg",
            "src=\"javascript",
            "src=\"vbscript",
            "src=\"data:text/html",
            "src=\"data:image/svg",
        ] {
            assert!(!html.contains(href), "renderer leaked href/src `{href}`");
        }
        // Blocked link/image markers must be present
        assert!(html.contains("nodx-blocked-link"));
        assert!(html.contains("nodx-blocked-image"));
    }

    #[test]
    fn navigation_rendering_fixture_is_deterministic() {
        let doc = parse_str(include_str!(
            "../../../spec/tests/rendering/navigation-toc.nodx"
        ));
        let html = render_html(&doc);
        assert!(html.contains("<nav id=\"primary-nav\" aria-label=\"Contents\">"));
        assert!(html.contains("<nav id=\"local-nav\" aria-label=\"In this section\">"));
        assert!(!html.contains("previous"));
        assert!(!html.contains("next"));
    }
}

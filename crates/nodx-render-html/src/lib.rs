#![forbid(unsafe_code)]

use nodx_core::{
    Document, Inline, NavigationGraph, Node, ResourceLimits, Value, default_navigation_label,
    parse_str, plain_inlines, resolve_navigation, sha256_bytes,
};
use nodx_style::{sanitize_stylesheet, yaml_style_to_css};
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
    let remote_assets = document_allows_remote_assets(doc);
    let policy = ResourcePolicy::new(options.limits).with_remote_assets(remote_assets);
    let body = render_body(doc, &navigation, policy);
    if !options.standalone {
        return body;
    }

    let stylesheet = base_stylesheet(doc);
    let page_stylesheet = page_stylesheet(doc, policy);
    let mut style_hashes = vec![sha256_base64_for_csp(stylesheet.as_bytes())];
    if !page_stylesheet.is_empty() {
        style_hashes.push(sha256_base64_for_csp(page_stylesheet.as_bytes()));
    }
    for component in component_definitions(doc) {
        if let Some(style) = component_style(component) {
            let sanitized = sanitize_stylesheet(style, policy.limits());
            style_hashes.push(sha256_base64_for_csp(sanitized.as_bytes()));
        }
    }
    for stylesheet in document_stylesheets(doc) {
        let sanitized = sanitize_stylesheet(stylesheet, policy.limits());
        style_hashes.push(sha256_base64_for_csp(sanitized.as_bytes()));
    }
    let inline_style_hashes = inline_style_hashes(doc);

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
        out.push_str("<meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; img-src 'self' data:");
        if remote_assets {
            out.push_str(" http: https:");
        }
        out.push_str("; media-src 'self'");
        if remote_assets {
            out.push_str(" http: https:");
        }
        out.push_str("; style-src 'sha256-");
        out.push_str(&style_hashes.join("' 'sha256-"));
        out.push('\'');
        if !inline_style_hashes.is_empty() {
            out.push_str(" 'unsafe-hashes'");
            for hash in inline_style_hashes {
                out.push_str(" 'sha256-");
                out.push_str(&hash);
                out.push('\'');
            }
        }
        out.push_str("; base-uri 'none'; form-action 'none'; frame-ancestors 'none'\">");
    }
    out.push_str("<style>");
    out.push_str(&stylesheet);
    out.push_str("</style>");
    if !page_stylesheet.is_empty() {
        out.push_str("<style>");
        out.push_str(&page_stylesheet);
        out.push_str("</style>");
    }

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
    for stylesheet in document_stylesheets(doc) {
        out.push_str("<style>");
        out.push_str(&sanitize_stylesheet(stylesheet, policy.limits()));
        out.push_str("</style>");
    }
    for component in component_definitions(doc) {
        if let Some(style) = component_style(component) {
            out.push_str("<style>");
            out.push_str(&sanitize_stylesheet(style, policy.limits()));
            out.push_str("</style>");
        }
    }
    if is_docs_layout(doc) {
        let mut content = String::new();
        for (i, node) in doc.body.iter().enumerate() {
            render_node(&mut content, node, &i.to_string(), navigation, policy, doc);
        }
        render_docs_body(&mut out, doc, navigation, &content);
        return out;
    }
    for (i, node) in doc.body.iter().enumerate() {
        render_node(&mut out, node, &i.to_string(), navigation, policy, doc);
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

fn document_allows_remote_assets(doc: &Document) -> bool {
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

fn base_stylesheet(doc: &Document) -> String {
    let theme = match doc.meta.get("theme") {
        Some(Value::String(theme)) => theme.as_str(),
        _ => "base",
    };
    match theme {
        "none" | "plain" => String::from("html[dir=\"rtl\"]{direction:rtl}"),
        "print" => {
            let mut css = String::from(standard_tokens());
            css.push_str(":root{--nodx-font-body:Georgia,\"Times New Roman\",serif;--nodx-font-heading:var(--nodx-font-body);--nodx-color-heading:#111827;--nodx-color-primary:#374151;--nodx-color-accent:#7f1d1d;--nodx-color-surface:#ffffff}body{font:11pt/1.55 var(--nodx-font-body);max-width:none;margin:0;color:var(--nodx-color-text);background:var(--nodx-color-bg)}@page{size:A4;margin:var(--nodx-page-margin)}h1,h2,h3{break-after:avoid}table,figure,aside,.nodx-callout{break-inside:avoid}.pagebreak{break-before:page;border:0;margin:0}");
            css.push_str(common_styles());
            css
        }
        "presentation" => {
            let mut css = String::from(standard_tokens());
            css.push_str(":root{--nodx-color-bg:#f8f7ff;--nodx-color-heading:#312e81;--nodx-color-primary:#7c3aed;--nodx-color-accent:#e11d48;--nodx-color-rule:#ddd6fe;--nodx-color-surface:#ffffff}body{font:28px/1.45 var(--nodx-font-body);max-width:1100px;margin:40px auto;padding:0 28px;color:var(--nodx-color-text);background:var(--nodx-color-bg)}h1{font-size:2.4em}h2{font-size:1.8em}");
            css.push_str(common_styles());
            css
        }
        "web" => {
            let mut css = String::from(standard_tokens());
            css.push_str(":root{--nodx-color-bg:#f8fafc;--nodx-color-heading:#0f172a;--nodx-color-primary:#2563eb;--nodx-color-accent:#be123c;--nodx-color-rule:#cbd5e1;--nodx-color-surface:#ffffff}body{font:16px/1.65 var(--nodx-font-body);max-width:960px;margin:32px auto;padding:0 18px;color:var(--nodx-color-text);background:var(--nodx-color-bg)}");
            css.push_str(common_styles());
            css
        }
        "docs" => {
            let mut css = String::from(standard_tokens());
            css.push_str(":root{--nodx-color-bg:#ffffff;--nodx-color-heading:#172554;--nodx-color-primary:#1d4ed8;--nodx-color-accent:#7c3aed;--nodx-color-rule:#dbe3ef;--nodx-color-surface:#f8fafc}");
            css.push_str(docs_styles());
            css.push_str(common_styles());
            css
        }
        _ => {
            let mut css = String::from(standard_tokens());
            css.push_str(":root{--nodx-color-heading:#111827;--nodx-color-surface:#f9fafb}body{font:16px/1.6 var(--nodx-font-body);max-width:920px;margin:32px auto;padding:0 16px;color:var(--nodx-color-text);background:var(--nodx-color-bg)}");
            css.push_str(common_styles());
            css
        }
    }
}

fn standard_tokens() -> &'static str {
    "html{font-family:system-ui}html[dir=\"rtl\"]{direction:rtl}:root{--nodx-color-text:#1f2937;--nodx-color-muted:#4b5563;--nodx-color-bg:#ffffff;--nodx-color-primary:#0f766e;--nodx-color-accent:#b91c1c;--nodx-color-rule:#e5e7eb;--nodx-color-surface:transparent;--nodx-font-body:system-ui,-apple-system,BlinkMacSystemFont,\"Segoe UI\",sans-serif;--nodx-font-heading:var(--nodx-font-body);--nodx-font-mono:ui-monospace,SFMono-Regular,Menlo,monospace;--nodx-page-margin:22mm;--nodx-line-height:1.6;--nodx-block-gap:1rem}"
}

fn common_styles() -> &'static str {
    concat!(
        "h1,h2,h3,h4,h5,h6{font-family:var(--nodx-font-heading);line-height:1.25;color:var(--nodx-color-heading,#0f172a);margin-top:1.4em}",
        "p{margin:0 0 1em}pre{padding:12px;background:#f5f5f5;overflow:auto;border-radius:6px}code{font-family:var(--nodx-font-mono)}",
        "aside{border-inline-start:4px solid #b57f00;padding:8px 12px;background:#fff8e6}",
        ".nodx-callout{margin:1em 0;padding:.85em 1em;border:1px solid var(--nodx-callout-border,#d1d5db);border-inline-start-width:4px;border-radius:8px;background:var(--nodx-callout-bg,#f8fafc);color:var(--nodx-color-text)}",
        ".nodx-callout__label{margin:0 0 .35em;font-size:.78em;font-weight:750;letter-spacing:.04em;text-transform:uppercase;color:var(--nodx-callout-fg,var(--nodx-color-muted))}",
        ".nodx-callout--note{--nodx-callout-border:#94a3b8;--nodx-callout-bg:#f8fafc;--nodx-callout-fg:#475569}.nodx-callout--info{--nodx-callout-border:#38bdf8;--nodx-callout-bg:#f0f9ff;--nodx-callout-fg:#0369a1}.nodx-callout--tip{--nodx-callout-border:#2dd4bf;--nodx-callout-bg:#f0fdfa;--nodx-callout-fg:#0f766e}",
        ".nodx-callout--important{--nodx-callout-border:#a78bfa;--nodx-callout-bg:#f5f3ff;--nodx-callout-fg:#6d28d9}.nodx-callout--caution{--nodx-callout-border:#f59e0b;--nodx-callout-bg:#fffbeb;--nodx-callout-fg:#b45309}.nodx-callout--warning{--nodx-callout-border:#f97316;--nodx-callout-bg:#fff7ed;--nodx-callout-fg:#c2410c}",
        ".nodx-callout--danger{--nodx-callout-border:#ef4444;--nodx-callout-bg:#fef2f2;--nodx-callout-fg:#b91c1c}.nodx-callout--example{--nodx-callout-border:#22c55e;--nodx-callout-bg:#f0fdf4;--nodx-callout-fg:#15803d}.nodx-callout--summary{--nodx-callout-border:#64748b;--nodx-callout-bg:#f8fafc;--nodx-callout-fg:#334155}",
        "table{border-collapse:collapse;margin:0 0 1em}caption{text-align:start;font-weight:600;margin-bottom:.35em}td,th{border:1px solid #d1d5db;padding:6px 10px}thead th{background:#f3f4f6;text-align:start}",
        "figure{margin:1.5em 0}figcaption{font-size:0.9em;color:var(--nodx-color-muted)}nav ol{padding-inline-start:1.5rem}nav strong{display:block;margin-bottom:0.4em}",
        ".nodx-grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(12rem,1fr));gap:var(--nodx-block-gap);margin:0 0 1em}.nodx-columns{columns:2 18rem;column-gap:2rem;margin:0 0 1em}.nodx-frame{border:1px solid var(--nodx-color-rule,#e5e7eb);padding:1rem;margin:0 0 1em;border-radius:6px;background:var(--nodx-color-surface,transparent)}",
        ".nodx-blocked-link,.nodx-blocked-image{color:var(--nodx-color-accent);text-decoration:line-through}.nodx-blocked-link{cursor:not-allowed}.mention{font-variant:all-small-caps}.pagebreak{border:none;border-top:1px dashed #9ca3af;margin:2em 0}.math-inline{background:#f3f4f6;padding:1px 4px;border-radius:3px}"
    )
}

fn docs_styles() -> &'static str {
    "body.nodx-docs-layout{font:16px/1.65 var(--nodx-font-body);color:var(--nodx-color-text);margin:0;display:grid;grid-template-columns:minmax(220px,280px) minmax(0,1fr) minmax(180px,240px);gap:0;min-height:100vh}.nodx-docs-sidebar,.nodx-docs-outline{position:sticky;top:0;height:100vh;overflow:auto;padding:24px 18px;border-color:#e5e7eb}.nodx-docs-sidebar{border-inline-end:1px solid #e5e7eb;background:#f8fafc}.nodx-docs-outline{border-inline-start:1px solid #e5e7eb;background:#fff}.nodx-docs-main{min-width:0;max-width:860px;width:100%;padding:32px 32px 64px;margin:0 auto}.nodx-docs-brand{display:block;font-weight:700;color:var(--nodx-color-text);text-decoration:none;margin-bottom:18px}.nodx-docs-layout nav ol{list-style:none;padding:0;margin:0}.nodx-docs-layout nav li{margin:2px 0}.nodx-docs-layout nav li[data-level=\"2\"]{padding-inline-start:12px}.nodx-docs-layout nav li[data-level=\"3\"],.nodx-docs-layout nav li[data-level=\"4\"],.nodx-docs-layout nav li[data-level=\"5\"],.nodx-docs-layout nav li[data-level=\"6\"]{padding-inline-start:22px}.nodx-docs-layout nav a{display:block;color:#374151;text-decoration:none;border-radius:6px;padding:4px 6px}.nodx-docs-layout nav a:hover{background:#eef2ff;color:#111827}@media(max-width:920px){body.nodx-docs-layout{display:block}.nodx-docs-sidebar,.nodx-docs-outline{position:static;height:auto;border:0;border-bottom:1px solid #e5e7eb}.nodx-docs-outline{display:none}.nodx-docs-main{padding:24px 18px 48px}}"
}

fn sha256_base64_for_csp(input: &[u8]) -> String {
    let digest = sha256_bytes(input);
    // CSP hash uses standard base64 with padding, not base64url
    base64_standard(&digest)
}

fn base64_standard(input: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
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
    doc: &Document,
) {
    if let Some(template) = component_template(doc, &node.node_type) {
        render_component_template(out, template, node, path, navigation, policy, doc);
        return;
    }
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
        "paragraph" => wrap_inlines(out, "p", node, path, navigation, policy, doc),
        "section" => wrap_children(out, "section", node, path, navigation, policy, doc),
        "note" | "info" | "tip" | "important" | "caution" | "warning" | "danger" | "example"
        | "summary" => render_callout(out, node, path, navigation, policy, doc),
        "quote" => wrap_children(out, "blockquote", node, path, navigation, policy, doc),
        "list" => {
            let tag = if node.attrs.get("kind").map(|s| s.as_str()) == Some("ordered") {
                "ol"
            } else {
                "ul"
            };
            out.push_str(tag_open(tag, node).as_str());
            for (i, child) in node.children.iter().enumerate() {
                render_node(out, child, &child_path(path, i), navigation, policy, doc);
            }
            out.push_str(&format!("</{}>", tag));
        }
        "item" => wrap_inlines(out, "li", node, path, navigation, policy, doc),
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
            let style_source;
            let raw_style = node.text.as_deref().unwrap_or("");
            let source = if node.attrs.get("format").map(String::as_str) == Some("yaml") {
                style_source = yaml_style_to_css(raw_style)
                    .unwrap_or_else(|_| "/* NODX-E027: invalid YAML style block */".to_string());
                style_source.as_str()
            } else {
                raw_style
            };
            out.push_str("<style>");
            out.push_str(&sanitize_stylesheet(source, policy.limits()));
            out.push_str("</style>");
        }
        "table" => render_table(out, node, path, navigation, policy, doc),
        "row" => wrap_children(out, "tr", node, path, navigation, policy, doc),
        "cell" => {
            let tag = if node.attrs.get("header").map(|s| s.as_str()) == Some("true") {
                "th"
            } else {
                "td"
            };
            wrap_inlines(out, tag, node, path, navigation, policy, doc);
        }
        "figure" => wrap_children(out, "figure", node, path, navigation, policy, doc),
        "caption" => wrap_inlines(out, "figcaption", node, path, navigation, policy, doc),
        "image" => {
            let alt = node.attrs.get("alt").map(String::as_str).unwrap_or("");
            let safe_src = node
                .attrs
                .get("src")
                .and_then(|s| safe_image_url_with_policy(policy, s));
            match safe_src {
                Some(src) => {
                    out.push_str("<img");
                    out.push_str(&html_attrs(node));
                    out.push_str(" src=\"");
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
        "form" => wrap_children(out, "dl", node, path, navigation, policy, doc),
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
            out.push_str(&html_attrs(node));
            out.push('>');
            let safe_src = node
                .attrs
                .get("src")
                .and_then(|src| policy.classify_uri(ReferenceKind::MediaFallback, src).ok())
                .map(|uri| uri.raw);
            if node.node_type == "media"
                && let Some(src) = &safe_src
            {
                out.push_str("<video controls src=\"");
                escape_attr(out, src);
                out.push_str("\">");
                render_media_fallback_content(out, node, path, navigation, policy, doc, Some(src));
                out.push_str("</video>");
            } else {
                out.push_str("<div class=\"media-fallback\">");
                render_media_fallback_content(
                    out,
                    node,
                    path,
                    navigation,
                    policy,
                    doc,
                    safe_src.as_ref(),
                );
                out.push_str("</div>");
            }
            for (i, child) in node.children.iter().enumerate() {
                if child.node_type != "media-fallback" {
                    render_node(out, child, &child_path(path, i), navigation, policy, doc);
                }
            }
            out.push_str("</figure>");
        }
        "bibliography" => wrap_children(out, "ol", node, path, navigation, policy, doc),
        "citation-entry" => wrap_inlines(out, "li", node, path, navigation, policy, doc),
        "speaker-notes" => {
            out.push_str("<aside");
            out.push_str(&html_id(node));
            out.push_str(" class=\"speaker-notes\" aria-label=\"Speaker notes\">");
            render_inlines(out, &node.inlines, policy);
            for (i, child) in node.children.iter().enumerate() {
                render_node(out, child, &child_path(path, i), navigation, policy, doc);
            }
            out.push_str("</aside>");
        }
        "grid" => wrap_layout_children(out, "nodx-grid", node, path, navigation, policy, doc),
        "columns" => wrap_layout_children(out, "nodx-columns", node, path, navigation, policy, doc),
        "frame" => wrap_layout_children(out, "nodx-frame", node, path, navigation, policy, doc),
        "page" => wrap_layout_children(out, "nodx-page", node, path, navigation, policy, doc),
        _ if node.node_type.contains('-') => {
            out.push_str("<section");
            out.push_str(&html_attrs(node));
            out.push_str(" class=\"nodx-component nodx-component--fallback\" data-component=\"");
            escape_attr(out, &node.node_type);
            out.push_str("\"><p class=\"nodx-component__title\">");
            escape_html(out, &format!("{} fallback", node.node_type));
            out.push_str("</p>");
            render_inlines(out, &node.inlines, policy);
            for (i, child) in node.children.iter().enumerate() {
                render_node(out, child, &child_path(path, i), navigation, policy, doc);
            }
            out.push_str("</section>");
        }
        _ => wrap_children(out, "div", node, path, navigation, policy, doc),
    }
}

fn render_table(
    out: &mut String,
    node: &Node,
    path: &str,
    navigation: &NavigationGraph,
    policy: ResourcePolicy,
    doc: &Document,
) {
    out.push_str(tag_open("table", node).as_str());
    if let Some(caption) = node
        .attrs
        .get("caption")
        .or_else(|| node.attrs.get("title"))
        .filter(|caption| !caption.trim().is_empty())
    {
        out.push_str("<caption>");
        escape_html(out, caption);
        out.push_str("</caption>");
    }
    for (i, child) in node.children.iter().enumerate() {
        render_node(out, child, &child_path(path, i), navigation, policy, doc);
    }
    out.push_str("</table>");
}

fn render_callout(
    out: &mut String,
    node: &Node,
    path: &str,
    navigation: &NavigationGraph,
    policy: ResourcePolicy,
    doc: &Document,
) {
    let callout_type = callout_type(node);
    let tag = if matches!(callout_type, "example" | "summary") {
        "section"
    } else {
        "aside"
    };
    let label = callout_label(node, callout_type);
    out.push('<');
    out.push_str(tag);
    out.push_str(&html_attrs_with_extra_class(
        node,
        &format!("nodx-callout nodx-callout--{callout_type}"),
        policy,
    ));
    append_callout_a11y_attr(out, node, &label);
    out.push_str("><p class=\"nodx-callout__label\">");
    escape_html(out, &label);
    out.push_str("</p>");
    for (i, child) in node.children.iter().enumerate() {
        render_node(out, child, &child_path(path, i), navigation, policy, doc);
    }
    out.push_str("</");
    out.push_str(tag);
    out.push('>');
}

fn append_callout_a11y_attr(out: &mut String, node: &Node, label: &str) {
    if node.attrs.contains_key("aria-label") || node.attrs.contains_key("aria-labelledby") {
        return;
    }
    out.push_str(" aria-label=\"");
    escape_attr(out, label);
    out.push('"');
}

fn callout_type(node: &Node) -> &str {
    let raw = if node.node_type == "note" {
        node.attrs.get("type").map(String::as_str).unwrap_or("note")
    } else {
        node.node_type.as_str()
    };
    match raw {
        "note" | "info" | "tip" | "important" | "caution" | "warning" | "danger" | "example"
        | "summary" => raw,
        _ => "note",
    }
}

fn callout_label(node: &Node, callout_type: &str) -> String {
    node.attrs.get("title").cloned().unwrap_or_else(|| {
        match callout_type {
            "note" => "Note",
            "info" => "Info",
            "tip" => "Tip",
            "important" => "Important",
            "caution" => "Caution",
            "warning" => "Warning",
            "danger" => "Danger",
            "example" => "Example",
            "summary" => "Summary",
            _ => "Note",
        }
        .to_string()
    })
}

fn wrap_layout_children(
    out: &mut String,
    class_name: &str,
    node: &Node,
    path: &str,
    navigation: &NavigationGraph,
    policy: ResourcePolicy,
    doc: &Document,
) {
    out.push_str("<div");
    out.push_str(&html_attrs_with_extra_class(node, class_name, policy));
    out.push('>');
    for (i, child) in node.children.iter().enumerate() {
        render_node(out, child, &child_path(path, i), navigation, policy, doc);
    }
    out.push_str("</div>");
}

fn wrap_children(
    out: &mut String,
    tag: &str,
    node: &Node,
    path: &str,
    navigation: &NavigationGraph,
    policy: ResourcePolicy,
    doc: &Document,
) {
    out.push_str(tag_open(tag, node).as_str());
    for (i, child) in node.children.iter().enumerate() {
        render_node(out, child, &child_path(path, i), navigation, policy, doc);
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
    doc: &Document,
) {
    out.push_str(tag_open(tag, node).as_str());
    render_inlines(out, &node.inlines, policy);
    for (i, child) in node.children.iter().enumerate() {
        render_node(out, child, &child_path(path, i), navigation, policy, doc);
    }
    out.push_str(&format!("</{}>", tag));
}

fn render_component_template(
    out: &mut String,
    template: &str,
    node: &Node,
    path: &str,
    navigation: &NavigationGraph,
    policy: ResourcePolicy,
    doc: &Document,
) {
    let source = substitute_template_vars(template, node, doc);
    let parsed = parse_str(&source);
    let mut rendered = String::new();
    for (i, child) in parsed.body.iter().enumerate() {
        render_node(
            &mut rendered,
            child,
            &format!("{path}.template.{}.{}", node.node_type, i),
            navigation,
            policy,
            doc,
        );
    }
    let mut children = String::new();
    render_inlines(&mut children, &node.inlines, policy);
    for (i, child) in node.children.iter().enumerate() {
        render_node(
            &mut children,
            child,
            &child_path(path, i),
            navigation,
            policy,
            doc,
        );
    }
    out.push_str(
        &rendered
            .replace("<p><var>vars.children</var></p>", &children)
            .replace("<var>vars.children</var>", &children),
    );
}

fn render_docs_body(out: &mut String, doc: &Document, navigation: &NavigationGraph, content: &str) {
    let fallback_entries = if navigation.navigations.is_empty() {
        collect_heading_entries(&doc.body)
    } else {
        Vec::new()
    };
    out.push_str("<body class=\"nodx-docs-layout\"><aside class=\"nodx-docs-sidebar\"><a class=\"nodx-docs-brand\" href=\"#\">");
    escape_html(out, &docs_title(doc));
    out.push_str("</a>");
    render_docs_nav(out, navigation, &fallback_entries, 2, false);
    out.push_str("</aside><main class=\"nodx-docs-main\">");
    out.push_str(content);
    out.push_str("</main><aside class=\"nodx-docs-outline\">");
    render_docs_nav(out, navigation, &fallback_entries, 6, true);
    out.push_str("</aside></body>");
}

fn render_docs_nav(
    out: &mut String,
    navigation: &NavigationGraph,
    fallback_entries: &[(String, String, usize)],
    max_level: usize,
    skip_level_one: bool,
) {
    out.push_str("<nav><ol>");
    if let Some(nav) = navigation.navigations.first() {
        for entry in &nav.entries {
            render_docs_nav_item(
                out,
                &entry.id,
                &entry.title,
                entry.level,
                max_level,
                skip_level_one,
            );
        }
    } else {
        for (id, title, level) in fallback_entries {
            render_docs_nav_item(out, id, title, *level, max_level, skip_level_one);
        }
    }
    out.push_str("</ol></nav>");
}

fn render_docs_nav_item(
    out: &mut String,
    id: &str,
    title: &str,
    level: usize,
    max_level: usize,
    skip_level_one: bool,
) {
    if level > max_level || skip_level_one && level <= 1 || !is_safe_fragment_id(id) {
        return;
    }
    out.push_str("<li data-level=\"");
    out.push_str(&level.to_string());
    out.push_str("\"><a href=\"#");
    escape_attr(out, id);
    out.push_str("\">");
    escape_html(out, title);
    out.push_str("</a></li>");
}

fn collect_heading_entries(nodes: &[Node]) -> Vec<(String, String, usize)> {
    let mut out = Vec::new();
    collect_heading_entries_at(nodes, &mut out);
    out
}

fn collect_heading_entries_at(nodes: &[Node], out: &mut Vec<(String, String, usize)>) {
    for node in nodes {
        if node.node_type == "heading" {
            let id = node.id.clone().unwrap_or_default();
            let title = plain_inlines(&node.inlines);
            let level = node
                .attrs
                .get("level")
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(1);
            out.push((id, title, level));
        }
        collect_heading_entries_at(&node.children, out);
    }
}

fn docs_title(doc: &Document) -> String {
    match doc.meta.get("title") {
        Some(Value::String(title)) => title.clone(),
        _ => derive_title(&doc.body).unwrap_or_else(|| "Documentation".to_string()),
    }
}

fn is_docs_layout(doc: &Document) -> bool {
    matches!(doc.meta.get("layout"), Some(Value::String(layout)) if layout == "docs")
        || matches!(doc.meta.get("theme"), Some(Value::String(theme)) if theme == "docs")
}

fn substitute_template_vars(source: &str, node: &Node, doc: &Document) -> String {
    let mut out = String::new();
    let mut rest = source;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else {
            out.push_str(&rest[start..]);
            return out;
        };
        let key = after[..end].trim();
        if key == "children" {
            out.push_str("{{children}}");
        } else if let Some(name) = key.strip_prefix("attrs.") {
            out.push_str(node.attrs.get(name).map(String::as_str).unwrap_or(""));
        } else if let Some(name) = key.strip_prefix("vars.") {
            out.push_str(meta_string_map(doc.meta.get("vars"), name).unwrap_or(""));
        } else {
            out.push_str(node.attrs.get(key).map(String::as_str).unwrap_or(""));
        }
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    out
}

fn component_definitions(doc: &Document) -> Vec<&std::collections::BTreeMap<String, Value>> {
    match doc.meta.get("components") {
        Some(Value::List(items)) => items
            .iter()
            .filter_map(|item| match item {
                Value::Map(map) if matches!(map.get("name"), Some(Value::String(_))) => Some(map),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn document_stylesheets(doc: &Document) -> Vec<&str> {
    match doc.meta.get("stylesheets") {
        Some(Value::List(items)) => items
            .iter()
            .filter_map(|item| match item {
                Value::String(css) => Some(css.as_str()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn component_template<'a>(doc: &'a Document, name: &str) -> Option<&'a str> {
    component_definitions(doc).into_iter().find_map(|component| {
        if matches!(component.get("name"), Some(Value::String(component_name)) if component_name == name) {
            match component.get("template") {
                Some(Value::String(template)) => Some(template.as_str()),
                _ => None,
            }
        } else {
            None
        }
    })
}

fn component_style(component: &std::collections::BTreeMap<String, Value>) -> Option<&str> {
    match component.get("style") {
        Some(Value::String(style)) => Some(style.as_str()),
        _ => None,
    }
}

fn inline_style_hashes(doc: &Document) -> Vec<String> {
    let mut values = Vec::new();
    collect_node_style_values(&doc.body, &mut values);
    values
        .into_iter()
        .map(|value| sha256_base64_for_csp(value.as_bytes()))
        .collect()
}

fn collect_node_style_values(nodes: &[Node], out: &mut Vec<String>) {
    for node in nodes {
        if !node.styles.is_empty() {
            out.push(style_attr(&node.styles));
        }
        collect_inline_style_values(&node.inlines, out);
        collect_node_style_values(&node.children, out);
    }
}

fn collect_inline_style_values(inlines: &[Inline], out: &mut Vec<String>) {
    for inline in inlines {
        match inline {
            Inline::Strong(children)
            | Inline::Em(children)
            | Inline::Strike(children)
            | Inline::Sub(children)
            | Inline::Sup(children) => collect_inline_style_values(children, out),
            Inline::Mark { children, attrs } => {
                if !attrs.styles.is_empty() {
                    out.push(style_attr(&attrs.styles));
                }
                collect_inline_style_values(children, out);
            }
            Inline::Link { label, .. } => collect_inline_style_values(label, out),
            Inline::Span { children, attrs } => {
                if !attrs.styles.is_empty() {
                    out.push(style_attr(&attrs.styles));
                }
                collect_inline_style_values(children, out);
            }
            Inline::Text(_)
            | Inline::Code(_)
            | Inline::Var { .. }
            | Inline::Ref { .. }
            | Inline::Mention { .. }
            | Inline::FootnoteRef { .. }
            | Inline::CitationRef { .. }
            | Inline::MathInline { .. } => {}
        }
    }
}

fn meta_string_map<'a>(value: Option<&'a Value>, key: &str) -> Option<&'a str> {
    match value {
        Some(Value::Map(map)) => match map.get(key) {
            Some(Value::String(value)) => Some(value.as_str()),
            _ => None,
        },
        _ => None,
    }
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

fn render_media_fallback_content(
    out: &mut String,
    node: &Node,
    path: &str,
    navigation: &NavigationGraph,
    policy: ResourcePolicy,
    doc: &Document,
    safe_src: Option<&String>,
) {
    if let Some((i, fallback)) = node
        .children
        .iter()
        .enumerate()
        .find(|(_, child)| child.node_type == "media-fallback")
    {
        render_inlines(out, &fallback.inlines, policy);
        for (child_index, child) in fallback.children.iter().enumerate() {
            let fallback_path = child_path(path, i);
            render_node(
                out,
                child,
                &child_path(&fallback_path, child_index),
                navigation,
                policy,
                doc,
            );
        }
        return;
    }
    escape_html(
        out,
        node.attrs
            .get("alt")
            .map(String::as_str)
            .unwrap_or(&node.node_type),
    );
    if safe_src.is_none()
        && let Some(raw) = node.attrs.get("src")
        && !raw.is_empty()
    {
        out.push_str(" - ");
        escape_html(out, raw);
    }
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
    if !node.styles.is_empty() {
        s.push_str(" style=\"");
        escape_attr(&mut s, &style_attr(&node.styles));
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
    append_safe_structural_attrs(&mut s, node);
    s
}

fn html_attrs_with_extra_class(node: &Node, extra: &str, policy: ResourcePolicy) -> String {
    let mut s = html_id(node);
    s.push_str(" class=\"");
    escape_attr(&mut s, extra);
    for class in &node.classes {
        s.push(' ');
        escape_attr(&mut s, class);
    }
    s.push('"');
    let background_image = safe_background_image_css(node.attrs.get("background"), policy);
    if !node.styles.is_empty() || background_image.is_some() {
        s.push_str(" style=\"");
        let mut style = style_attr(&node.styles);
        if let Some(bg) = background_image {
            if !style.is_empty() {
                style.push_str("; ");
            }
            style.push_str(&bg);
        }
        escape_attr(&mut s, &style);
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
    append_safe_structural_attrs(&mut s, node);
    s
}

fn page_stylesheet(doc: &Document, policy: ResourcePolicy) -> String {
    let Some(Value::Map(page)) = doc.meta.get("page") else {
        return String::new();
    };
    let mut rules = Vec::new();
    if let Some(Value::String(bg)) = page
        .get("bg")
        .or_else(|| page.get("background-color"))
        .filter(|value| matches!(value, Value::String(_)))
        && safe_quick_style_value(bg)
    {
        rules.push(format!("background-color:{bg}"));
    }
    if let Some(Value::String(color)) = page.get("color")
        && safe_quick_style_value(color)
    {
        rules.push(format!("color:{color}"));
    }
    if let Some(bg) = page
        .get("background")
        .or_else(|| page.get("background-image"))
        .and_then(|value| match value {
            Value::String(raw) => safe_background_image_css(Some(raw), policy),
            _ => None,
        })
    {
        rules.push(bg);
    }
    if rules.is_empty() {
        String::new()
    } else {
        format!("body{{{}}}", rules.join(";"))
    }
}

fn safe_background_image_css(raw: Option<&String>, policy: ResourcePolicy) -> Option<String> {
    let raw = raw?;
    let safe = policy.classify_uri(ReferenceKind::Asset, raw).ok()?.raw;
    if safe.contains(['\'', '"', '(', ')', '\\']) {
        return None;
    }
    Some(format!("background-image:url('{safe}')"))
}

fn safe_quick_style_value(value: &str) -> bool {
    value.len() <= 240
        && !value
            .chars()
            .any(|c| matches!(c, '<' | '>' | '{' | '}' | ';'))
}

fn append_safe_structural_attrs(out: &mut String, node: &Node) {
    for (key, value) in &node.attrs {
        if !safe_structural_attr(key, value) {
            continue;
        }
        out.push(' ');
        out.push_str(key);
        out.push_str("=\"");
        escape_attr(out, value);
        out.push('"');
    }
}

fn safe_structural_attr(key: &str, value: &str) -> bool {
    match key {
        "role" => value.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'),
        "scope" => matches!(value, "col" | "row" | "colgroup" | "rowgroup"),
        "align" => matches!(value, "left" | "center" | "right" | "start" | "end"),
        "valign" => matches!(value, "top" | "middle" | "bottom" | "baseline"),
        "colspan" | "rowspan" => value
            .parse::<u16>()
            .is_ok_and(|number| (1..=1000).contains(&number)),
        _ if key.starts_with("data-") || key.starts_with("aria-") => value.len() <= 240,
        _ => false,
    }
}

fn style_attr(styles: &std::collections::BTreeMap<String, String>) -> String {
    let mut out = String::new();
    for (i, (key, value)) in styles.iter().enumerate() {
        if i > 0 {
            out.push_str("; ");
        }
        out.push_str(key);
        out.push_str(": ");
        out.push_str(value);
    }
    out
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
            Inline::Link {
                label,
                target,
                attrs,
            } => match safe_link_url_with_policy(policy, target) {
                Some(safe) => {
                    out.push_str("<a href=\"");
                    escape_attr(out, &safe);
                    out.push('"');
                    if let Some(title) = attrs.attrs.get("title") {
                        out.push_str(" title=\"");
                        escape_attr(out, title);
                        out.push('"');
                    }
                    if let Some(download) = attrs.attrs.get("download") {
                        out.push_str(" download=\"");
                        escape_attr(out, download);
                        out.push('"');
                    }
                    let rel = attrs
                        .attrs
                        .get("rel")
                        .map(String::as_str)
                        .unwrap_or("noopener noreferrer");
                    out.push_str(" rel=\"");
                    escape_attr(out, rel);
                    out.push_str("\">");
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
                if !attrs.styles.is_empty() {
                    out.push_str(" style=\"");
                    escape_attr(out, &style_attr(&attrs.styles));
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
            Inline::Mark { children, attrs } => {
                out.push_str("<mark");
                render_inline_attrs(out, attrs);
                out.push('>');
                render_inlines(out, children, policy);
                out.push_str("</mark>");
            }
            Inline::Strike(children) => {
                out.push_str("<s>");
                render_inlines(out, children, policy);
                out.push_str("</s>");
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

fn render_inline_attrs(out: &mut String, attrs: &nodx_core::Attrs) {
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
    if !attrs.styles.is_empty() {
        out.push_str(" style=\"");
        escape_attr(out, &style_attr(&attrs.styles));
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
        let doc = parse_str(":::style\na:hover { color: red; }\np { color: blue; }\n:::\n");
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
    fn table_caption_spans_and_layout_blocks_render_to_html() {
        let doc = parse_str(
            ":::table {caption=\"Revenue\"}\n:::row\n:::cell {header=\"true\" colspan=2 align=\"center\"}\nTotal\n:::\n:::\n:::\n\n:::grid {gap=\"2rem\"}\n:::frame {bg=\"#f8fafc\"}\nA\n:::\n:::\n",
        );
        let html = render_html(&doc);
        assert!(html.contains("<caption>Revenue</caption>"));
        assert!(html.contains("colspan=\"2\""));
        assert!(html.contains("align=\"center\""));
        assert!(html.contains("class=\"nodx-grid\""));
        assert!(html.contains("class=\"nodx-frame\""));
        assert!(html.contains("gap: 2rem"));
        assert!(html.contains("background-color: #f8fafc"));
    }

    #[test]
    fn quick_mark_strike_and_page_background_render_to_html() {
        let doc = parse_str(
            "---\nschema: nodx/1.0\npage:\n  bg: \"#101827\"\n  color: \"#f8fafc\"\n  background: \"assets/bg.png\"\n---\n\n==Marked=={bg=\"#ffe08a\" color=\"#111827\"} and ~~removed~~.\n\n:::page {bg=\"#ffffff\" background=\"assets/page.png\"}\nPage body.\n:::\n",
        );
        let html = render_html(&doc);
        assert!(html.contains(
            "body{background-color:#101827;color:#f8fafc;background-image:url('assets/bg.png')}"
        ));
        assert!(
            html.contains(
                "<mark style=\"background-color: #ffe08a; color: #111827\">Marked</mark>"
            )
        );
        assert!(html.contains("<s>removed</s>"));
        assert!(html.contains("class=\"nodx-page\""));
        assert!(html.contains("background-image:url(&#x27;assets/page.png&#x27;)"));
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
            for tag in ["<script", "<iframe", "<object", "<embed", "<svg"] {
                assert!(!html.contains(tag), "{:?} leaked {tag}", path.file_name());
            }
            for href in [
                "href=\"javascript",
                "href=\"vbscript",
                "src=\"javascript",
                "src=\"vbscript",
                "src=\"data:text/html",
                "src=\"data:image/svg",
            ] {
                assert!(!html.contains(href), "{:?} leaked {href}", path.file_name());
            }
            count += 1;
        }
        assert!(
            count >= 40,
            "expected at least 40 XSS fixtures, found {count}"
        );
    }

    #[test]
    fn xss_payload_corpus_is_neutralized() {
        let doc = parse_str(include_str!(
            "../../../spec/tests/security/xss/payloads.nodx"
        ));
        let html = render_html(&doc).to_ascii_lowercase();
        // No executable HTML elements
        for tag in [
            "<script",
            "<iframe",
            "<object",
            "<embed",
            "<svg",
            "<img src=x",
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
    fn remote_assets_render_when_declared() {
        let doc = parse_str(
            "---\nschema: nodx/1.0\nfeatures:\n  remote-assets: true\n---\n\n:::image {src=\"https://example.test/a.png\" alt=\"A\"}\n:::\n\n:::media {src=\"https://example.test/a.mp4\" alt=\"A\"}\n:::\n",
        );
        let html = render_html(&doc);
        assert!(html.contains("src=\"https://example.test/a.png\""));
        assert!(html.contains("<video controls src=\"https://example.test/a.mp4\""));
        assert!(html.contains("img-src 'self' data: http: https:"));
        assert!(html.contains("media-src 'self' http: https:"));
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

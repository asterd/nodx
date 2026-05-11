use crate::ast::{Document, Inline, Node, Value};
use crate::navigation::{NavigationGraph, resolve_navigation};
use crate::style_baseline::strip_forbidden_nods;

pub fn render_html(doc: &Document) -> String {
    let navigation = resolve_navigation(doc);
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
    let mut out = format!(
        "<!doctype html><html{}><meta charset=\"utf-8\"><style>html{{font-family:system-ui}}html[dir=\"rtl\"]{{direction:rtl}}body{{font:16px system-ui;line-height:1.6;max-width:900px;margin:32px auto;padding:0 16px}}pre{{padding:12px;background:#f5f5f5;overflow:auto}}aside{{border-inline-start:4px solid #b57f00;padding:8px 12px;background:#fff8e6}}table{{border-collapse:collapse}}td,th{{border:1px solid #ccc;padding:4px 8px}}.nodx-blocked-link,.nodx-blocked-image{{color:#b91c1c;text-decoration:line-through}}</style>",
        html_attrs,
    );
    if let Some(Value::String(title)) = doc.meta.get("title") {
        out.push_str("<title>");
        escape_html(&mut out, title);
        out.push_str("</title>");
    }
    for (i, node) in doc.body.iter().enumerate() {
        render_node(&mut out, node, &i.to_string(), &navigation);
    }
    out.push_str("</html>");
    out
}

fn render_node(out: &mut String, node: &Node, path: &str, navigation: &NavigationGraph) {
    match node.node_type.as_str() {
        "heading" => {
            let level = node
                .attrs
                .get("level")
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(1)
                .clamp(1, 6);
            out.push_str(&format!("<h{}{}>", level, html_id(node)));
            render_inlines(out, &node.inlines);
            out.push_str(&format!("</h{}>", level));
        }
        "paragraph" => wrap_inlines(out, "p", node, path, navigation),
        "section" => wrap_children(out, "section", node, path, navigation),
        "note" => wrap_children(out, "aside", node, path, navigation),
        "quote" => wrap_children(out, "blockquote", node, path, navigation),
        "list" => {
            let tag = if node.attrs.get("kind").map(|s| s.as_str()) == Some("ordered") {
                "ol"
            } else {
                "ul"
            };
            out.push_str(tag_open(tag, node).as_str());
            for (i, child) in node.children.iter().enumerate() {
                render_node(out, child, &child_path(path, i), navigation);
            }
            out.push_str(&format!("</{}>", tag));
        }
        "item" => wrap_inlines(out, "li", node, path, navigation),
        "code" | "pre" => {
            out.push_str("<pre><code>");
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
            escape_style(out, node.text.as_deref().unwrap_or(""));
            out.push_str("</style>");
        }
        "table" => wrap_children(out, "table", node, path, navigation),
        "row" => wrap_children(out, "tr", node, path, navigation),
        "cell" => {
            let tag = if node.attrs.get("header").map(|s| s.as_str()) == Some("true") {
                "th"
            } else {
                "td"
            };
            wrap_inlines(out, tag, node, path, navigation);
        }
        "figure" => wrap_children(out, "figure", node, path, navigation),
        "caption" => wrap_inlines(out, "figcaption", node, path, navigation),
        "image" => {
            let alt = node.attrs.get("alt").map(String::as_str).unwrap_or("");
            let safe_src = node.attrs.get("src").and_then(|s| {
                if is_relative_asset(s) {
                    Some(s.clone())
                } else {
                    safe_image_url(s)
                }
            });
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
        "form" => wrap_children(out, "dl", node, path, navigation),
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
            let label = nav
                .map(|nav| nav.label.as_str())
                .unwrap_or("Table of contents");
            out.push_str(" aria-label=\"");
            escape_attr(out, label);
            out.push_str("\"><strong>");
            escape_html(out, label);
            out.push_str("</strong>");
            if let Some(nav) = nav {
                if !nav.entries.is_empty() {
                    out.push_str("<ol>");
                    for entry in &nav.entries {
                        out.push_str("<li><a href=\"#");
                        escape_attr(out, &entry.id);
                        out.push_str("\">");
                        escape_html(out, &entry.title);
                        out.push_str("</a></li>");
                    }
                    out.push_str("</ol>");
                }
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
            if let Some(src) = node.attrs.get("src") {
                out.push_str(" - ");
                escape_html(out, src);
            }
            out.push_str("</div>");
            for (i, child) in node.children.iter().enumerate() {
                render_node(out, child, &child_path(path, i), navigation);
            }
            out.push_str("</figure>");
        }
        "bibliography" => wrap_children(out, "ol", node, path, navigation),
        "citation-entry" => wrap_inlines(out, "li", node, path, navigation),
        _ => wrap_children(out, "div", node, path, navigation),
    }
}

fn wrap_children(
    out: &mut String,
    tag: &str,
    node: &Node,
    path: &str,
    navigation: &NavigationGraph,
) {
    out.push_str(tag_open(tag, node).as_str());
    for (i, child) in node.children.iter().enumerate() {
        render_node(out, child, &child_path(path, i), navigation);
    }
    out.push_str(&format!("</{}>", tag));
}

fn wrap_inlines(
    out: &mut String,
    tag: &str,
    node: &Node,
    path: &str,
    navigation: &NavigationGraph,
) {
    out.push_str(tag_open(tag, node).as_str());
    render_inlines(out, &node.inlines);
    for (i, child) in node.children.iter().enumerate() {
        render_node(out, child, &child_path(path, i), navigation);
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
    if let Some(dir) = node.attrs.get("dir") {
        if matches!(dir.as_str(), "ltr" | "rtl" | "auto") {
            s.push_str(" dir=\"");
            escape_attr(&mut s, dir);
            s.push('"');
        }
    }
    if let Some(title) = node.attrs.get("title") {
        s.push_str(" title=\"");
        escape_attr(&mut s, title);
        s.push('"');
    }
    s
}

fn render_inlines(out: &mut String, inlines: &[Inline]) {
    for item in inlines {
        match item {
            Inline::Text(text) => escape_html(out, text),
            Inline::Strong(children) => {
                out.push_str("<strong>");
                render_inlines(out, children);
                out.push_str("</strong>");
            }
            Inline::Em(children) => {
                out.push_str("<em>");
                render_inlines(out, children);
                out.push_str("</em>");
            }
            Inline::Code(text) => {
                out.push_str("<code>");
                escape_html(out, text);
                out.push_str("</code>");
            }
            Inline::Link { label, target } => match safe_link_url(target) {
                Some(safe) => {
                    out.push_str("<a href=\"");
                    escape_attr(out, &safe);
                    out.push_str("\" rel=\"noopener noreferrer\">");
                    render_inlines(out, label);
                    out.push_str("</a>");
                }
                None => {
                    out.push_str("<a class=\"nodx-blocked-link\" data-blocked=\"");
                    escape_attr(out, target);
                    out.push_str("\" title=\"Blocked unsafe URL\">");
                    render_inlines(out, label);
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
                if let Some(dir) = attrs.attrs.get("dir") {
                    if matches!(dir.as_str(), "ltr" | "rtl" | "auto") {
                        out.push_str(" dir=\"");
                        escape_attr(out, dir);
                        out.push('"');
                    }
                }
                if let Some(title) = attrs.attrs.get("title") {
                    out.push_str(" title=\"");
                    escape_attr(out, title);
                    out.push('"');
                }
                out.push('>');
                render_inlines(out, children);
                out.push_str("</span>");
            }
            Inline::Mark(children) => {
                out.push_str("<mark>");
                render_inlines(out, children);
                out.push_str("</mark>");
            }
            Inline::Sub(children) => {
                out.push_str("<sub>");
                render_inlines(out, children);
                out.push_str("</sub>");
            }
            Inline::Sup(children) => {
                out.push_str("<sup>");
                render_inlines(out, children);
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
                out.push_str("<a href=\"#");
                escape_attr(out, target);
                out.push_str("\">@");
                escape_html(out, target);
                out.push_str("</a>");
            }
            Inline::Mention { kind, target } => {
                out.push_str("<span class=\"mention\">@");
                escape_html(out, kind);
                out.push(':');
                escape_html(out, target);
                out.push_str("</span>");
            }
            Inline::FootnoteRef { target } | Inline::CitationRef { target } => {
                out.push_str("<a href=\"#");
                escape_attr(out, target);
                out.push_str("\">[");
                escape_html(out, target);
                out.push_str("]</a>");
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
            _ => out.push(ch),
        }
    }
}

const ALLOWED_LINK_SCHEMES: &[&str] = &["http", "https", "mailto"];
const ALLOWED_IMAGE_SCHEMES: &[&str] = &["http", "https"];
const ALLOWED_IMAGE_DATA: &[&str] = &[
    "data:image/png",
    "data:image/jpeg",
    "data:image/webp",
    "data:image/svg+xml",
];

pub fn safe_link_url(raw: &str) -> Option<String> {
    classify_url(raw, ALLOWED_LINK_SCHEMES, &[])
}

pub fn safe_image_url(raw: &str) -> Option<String> {
    classify_url(raw, ALLOWED_IMAGE_SCHEMES, ALLOWED_IMAGE_DATA)
}

fn is_relative_asset(raw: &str) -> bool {
    is_safe_asset_ref(raw)
}

pub fn is_safe_asset_ref(raw: &str) -> bool {
    let trimmed = raw.trim();
    if trimmed.is_empty()
        || trimmed.starts_with('/')
        || trimmed.contains('\\')
        || trimmed
            .chars()
            .any(|c| (c as u32) < 0x20 || c == '\u{007f}')
    {
        return false;
    }
    if trimmed.starts_with('#') {
        return true;
    }
    let scheme_end = trimmed.find(|c: char| !is_scheme_char(c));
    if matches!(scheme_end, Some(i) if i > 0 && trimmed[i..].starts_with(':')) {
        return false;
    }
    trimmed
        .split('/')
        .all(|part| !part.is_empty() && part != "." && part != "..")
}

fn classify_url(raw: &str, schemes: &[&str], data_prefixes: &[&str]) -> Option<String> {
    let trimmed = raw.trim_matches(|c: char| c.is_ascii_whitespace());
    if trimmed.is_empty() {
        return None;
    }
    if trimmed
        .chars()
        .any(|c| (c as u32) < 0x20 || c == '\u{007f}')
    {
        return None;
    }
    if trimmed.starts_with('#') || trimmed.starts_with('/') {
        return Some(trimmed.to_string());
    }
    let scheme_end = trimmed.find(|c: char| !is_scheme_char(c));
    let has_scheme = matches!(scheme_end, Some(i) if i > 0 && trimmed[i..].starts_with(':'));
    if !has_scheme {
        return Some(trimmed.to_string());
    }
    let scheme_decoded = percent_decode_ascii(&trimmed[..scheme_end.unwrap()]).to_ascii_lowercase();
    if scheme_decoded.is_empty() {
        return None;
    }
    if schemes.iter().any(|s| *s == scheme_decoded) {
        return Some(trimmed.to_string());
    }
    if scheme_decoded == "data" {
        let lower = trimmed.to_ascii_lowercase();
        if data_prefixes.iter().any(|p| lower.starts_with(p)) {
            return Some(trimmed.to_string());
        }
    }
    None
}

fn is_scheme_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.' | '%')
}

fn percent_decode_ascii(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = hex_value(bytes[i + 1]);
            let lo = hex_value(bytes[i + 2]);
            if let (Some(h), Some(l)) = (hi, lo) {
                let v = (h << 4) | l;
                if v < 0x80 {
                    out.push(v as char);
                    i += 3;
                    continue;
                }
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn hex_value(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn escape_attr(out: &mut String, input: &str) {
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            _ => out.push(ch),
        }
    }
}

fn escape_style(out: &mut String, input: &str) {
    let lower = input.to_ascii_lowercase();
    if lower.contains("</style") || lower.contains("expression(") || lower.contains("javascript:") {
        out.push_str("/* nodx: blocked unsafe style content */");
        return;
    }
    let scrubbed = strip_forbidden_nods(input);
    for ch in scrubbed.chars() {
        match ch {
            '<' => out.push_str("\\3C "),
            '>' => out.push_str("\\3E "),
            _ => out.push(ch),
        }
    }
}

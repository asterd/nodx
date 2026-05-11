use crate::ast::{Document, Inline, Node, Value};
use crate::inline_parser::{plain_inlines, plain_node_text};

pub fn render_tui(doc: &Document) -> String {
    let mut out = String::new();
    let ansi = ansi_enabled();
    let title = match doc.meta.get("title") {
        Some(Value::String(s)) => Some(s.clone()),
        _ => derive_title(&doc.body),
    };
    if let Some(title) = title {
        out.push_str(&paint(ansi, "1;36", &title));
        out.push_str("\n");
        out.push_str(&paint(ansi, "2", &"═".repeat(title.chars().count().max(8))));
        out.push_str("\n\n");
    }
    for node in &doc.body {
        render_tui_node(&mut out, node, 0, ansi);
    }
    out
}

fn derive_title(nodes: &[Node]) -> Option<String> {
    for node in nodes {
        if node.node_type == "heading" {
            let mut acc = String::new();
            for inline in &node.inlines {
                if let Inline::Text(t) = inline {
                    acc.push_str(t);
                }
            }
            if !acc.trim().is_empty() {
                return Some(acc);
            }
        }
        if let Some(t) = derive_title(&node.children) {
            return Some(t);
        }
    }
    None
}

fn render_tui_node(out: &mut String, node: &Node, indent: usize, ansi: bool) {
    let pad = " ".repeat(indent);
    match node.node_type.as_str() {
        "heading" => {
            let level = node
                .attrs
                .get("level")
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(1)
                .clamp(1, 6);
            let marker = if level == 1 { "█" } else { "▸" };
            let color = if level == 1 { "1;36" } else { "1;34" };
            out.push_str(&format!(
                "{}{} {}\n",
                pad,
                paint(ansi, color, marker),
                paint(ansi, color, &plain_inlines(&node.inlines))
            ));
            if level == 1 {
                out.push_str(&format!(
                    "{}{}\n",
                    pad,
                    paint(
                        ansi,
                        "2",
                        &"─".repeat(plain_inlines(&node.inlines).chars().count().max(8))
                    )
                ));
            }
            out.push('\n');
        }
        "paragraph" => {
            out.push_str(&wrap_text(&plain_inlines(&node.inlines), indent, 96));
            out.push('\n');
        }
        "item" => {
            let bullet = match node.attrs.get("checked").map(String::as_str) {
                Some("true") => paint(ansi, "32", "☑"),
                Some("false") => paint(ansi, "33", "☐"),
                _ => paint(ansi, "36", "•"),
            };
            out.push_str(&format!(
                "{}{} {}\n",
                pad,
                bullet,
                plain_inlines(&node.inlines)
            ));
            for child in &node.children {
                render_tui_node(out, child, indent + 2, ansi);
            }
        }
        "code" | "pre" | "math" => render_tui_box(
            out,
            indent,
            &node.node_type,
            node.text.as_deref().unwrap_or(""),
            ansi,
        ),
        "cell" => out.push_str(&plain_node_text(node)),
        "row" => {
            out.push_str(&paint(ansi, "2", &pad));
            out.push('│');
            for (i, cell) in node.children.iter().enumerate() {
                if i > 0 {
                    out.push_str(" │ ");
                }
                render_tui_node(out, cell, 0, ansi);
            }
            out.push('│');
            out.push('\n');
        }
        "table" => {
            out.push_str(&format!("{}{}\n", pad, paint(ansi, "2", "┌─ table ─")));
            for child in &node.children {
                render_tui_node(out, child, indent, ansi);
            }
            out.push_str(&format!("{}{}\n\n", pad, paint(ansi, "2", "└─────────")));
        }
        "image" => {
            let alt = node.attrs.get("alt").map(String::as_str).unwrap_or("image");
            let src = node.attrs.get("src").map(String::as_str).unwrap_or("");
            out.push_str(&format!(
                "{}{} {} {}\n\n",
                pad,
                paint(ansi, "35", "◼"),
                alt,
                paint(ansi, "2", src)
            ));
        }
        "caption" => out.push_str(&format!(
            "{}{}\n\n",
            pad,
            paint(ansi, "2", &plain_inlines(&node.inlines))
        )),
        "form" => {
            out.push_str(&format!("{}{}\n", pad, paint(ansi, "1;36", "╭─ form")));
            for child in &node.children {
                render_tui_node(out, child, indent + 2, ansi);
            }
            out.push_str(&format!("{}{}\n\n", pad, paint(ansi, "1;36", "╰─")));
        }
        "field" => {
            let label = node
                .attrs
                .get("label")
                .or_else(|| node.attrs.get("name"))
                .map(String::as_str)
                .unwrap_or("Field");
            let value = node.attrs.get("value").map(String::as_str).unwrap_or("");
            out.push_str(&format!("{}{} {}\n", pad, paint(ansi, "2", label), value));
        }
        "toc" => out.push_str(&format!(
            "{}{}\n\n",
            pad,
            paint(ansi, "2", "◦ table of contents placeholder")
        )),
        "pagebreak" => out.push_str(&format!(
            "{}{}\n\n",
            pad,
            paint(ansi, "2", "╌ page break ╌")
        )),
        "media" | "embed" => {
            let alt = node
                .attrs
                .get("alt")
                .map(String::as_str)
                .unwrap_or(&node.node_type);
            let src = node.attrs.get("src").map(String::as_str).unwrap_or("");
            out.push_str(&format!(
                "{}{} {} {}\n",
                pad,
                paint(ansi, "35", "▣"),
                alt,
                paint(ansi, "2", src)
            ));
            for child in &node.children {
                render_tui_node(out, child, indent + 2, ansi);
            }
            out.push('\n');
        }
        "bibliography" => {
            out.push_str(&format!("{}{}\n", pad, paint(ansi, "1;34", "Bibliography")));
            for child in &node.children {
                render_tui_node(out, child, indent + 2, ansi);
            }
            out.push('\n');
        }
        "citation-entry" => out.push_str(&format!(
            "{}{} {}\n",
            pad,
            paint(ansi, "36", "•"),
            plain_node_text(node)
        )),
        _ => {
            if node.node_type == "note" {
                let label = node.attrs.get("type").map(String::as_str).unwrap_or("note");
                out.push_str(&format!(
                    "{}{} {}\n",
                    pad,
                    paint(ansi, "33", "╭─"),
                    paint(ansi, "1;33", label)
                ));
            } else if node.node_type.contains('-') {
                out.push_str(&format!(
                    "{}{} {}\n",
                    pad,
                    paint(ansi, "35", "◇"),
                    paint(ansi, "35", &node.node_type)
                ));
            }
            for child in &node.children {
                render_tui_node(
                    out,
                    child,
                    indent + if node.node_type == "list" { 0 } else { 2 },
                    ansi,
                );
            }
            if node.node_type == "list" {
                out.push('\n');
            } else if node.node_type == "note" {
                out.push_str(&format!("{}{}\n\n", pad, paint(ansi, "33", "╰─")));
            }
        }
    }
}

fn ansi_enabled() -> bool {
    std::env::var_os("NO_COLOR").is_none()
        && std::env::var("TERM")
            .map(|term| term != "dumb")
            .unwrap_or(true)
}

fn paint(ansi: bool, code: &str, text: &str) -> String {
    if ansi {
        format!("\x1b[{}m{}\x1b[0m", code, text)
    } else {
        text.to_string()
    }
}

fn render_tui_box(out: &mut String, indent: usize, title: &str, text: &str, ansi: bool) {
    let pad = " ".repeat(indent);
    let title = format!(" {} ", title);
    out.push_str(&format!(
        "{}╭─{}{}\n",
        pad,
        paint(ansi, "1;35", &title),
        "─".repeat(12)
    ));
    for line in text.lines() {
        out.push_str(&format!("{}│ {}\n", pad, paint(ansi, "37", line)));
    }
    out.push_str(&format!(
        "{}╰{}\n\n",
        pad,
        "─".repeat(16 + title.chars().count())
    ));
}

fn wrap_text(input: &str, indent: usize, width: usize) -> String {
    let pad = " ".repeat(indent);
    let mut out = String::new();
    let mut line = String::new();
    let limit = width.saturating_sub(indent).max(24);
    for word in input.split_whitespace() {
        if !line.is_empty() && line.chars().count() + word.chars().count() + 1 > limit {
            out.push_str(&pad);
            out.push_str(line.trim_end());
            out.push('\n');
            line.clear();
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        out.push_str(&pad);
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out
}

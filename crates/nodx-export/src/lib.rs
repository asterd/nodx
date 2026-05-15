#![forbid(unsafe_code)]

use nodx_core::{Document, Inline, Node, ResourceLimits, Value, crc32 as core_crc32};
use nodx_render_html::render_html;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExportFormat {
    Pdf,
    Docx,
    Pptx,
    Markdown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Exported {
    pub bytes: Vec<u8>,
    pub loss_report: LossReport,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LossReport {
    pub schema: String,
    pub source_schema: String,
    pub profile: String,
    pub format: String,
    pub lossy: bool,
    pub losses: Vec<Loss>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Loss {
    pub code: String,
    pub severity: String,
    pub path: String,
    pub message: String,
}

pub const PRESENTATION_PROFILE_NAME: &str = "NODX-Presentation-1.2";

pub fn presentation_profile() -> &'static str {
    PRESENTATION_PROFILE_NAME
}

pub fn export_document(doc: &Document, format: ExportFormat) -> Exported {
    export_document_with_limits(doc, format, ResourceLimits::default())
}

pub fn export_document_with_limits(
    doc: &Document,
    format: ExportFormat,
    limits: ResourceLimits,
) -> Exported {
    match format {
        ExportFormat::Pdf => export_pdf_bridge(doc),
        ExportFormat::Docx => export_docx_with_limits(doc, limits),
        ExportFormat::Pptx => export_pptx_with_limits(doc, limits),
        ExportFormat::Markdown => export_markdown(doc),
    }
}

pub fn export_markdown(doc: &Document) -> Exported {
    let mut report = base_report(doc, "markdown");
    let mut out = String::new();
    push_markdown_nodes(&mut out, &doc.body, "$.body", &mut report, 0);
    if !out.ends_with('\n') {
        out.push('\n');
    }
    Exported {
        bytes: out.into_bytes(),
        loss_report: finish_report(report),
    }
}

pub fn markdown_to_nodx(source: &str) -> Exported {
    let mut report = LossReport {
        schema: "nodx/export-loss/1.2".to_string(),
        source_schema: "markdown".to_string(),
        profile: "NODX-Markdown-Bridge-1.0".to_string(),
        format: "nodx".to_string(),
        lossy: false,
        losses: Vec::new(),
    };
    let converted = convert_markdown_body(source, &mut report);
    Exported {
        bytes: converted.into_bytes(),
        loss_report: finish_report(report),
    }
}

pub fn export_pdf_bridge(doc: &Document) -> Exported {
    let mut report = base_report(doc, "pdf");
    report.losses.push(loss(
        "NODX-E026",
        "warning",
        "$",
        "PDF export is a safe paged HTML host bridge; a host renderer must produce final PDF bytes.",
    ));
    let html = paged_html_bridge(doc);
    Exported {
        bytes: html.into_bytes(),
        loss_report: finish_report(report),
    }
}

pub fn export_docx(doc: &Document) -> Exported {
    export_docx_with_limits(doc, ResourceLimits::default())
}

pub fn export_docx_with_limits(doc: &Document, limits: ResourceLimits) -> Exported {
    let mut report = base_report(doc, "docx");
    collect_common_losses(&doc.body, "$.body", &mut report);
    report.losses.push(loss(
        "NODX-E026",
        "warning",
        "$",
        "DOCX export preserves text structure only; NODX semantics remain in the source document.",
    ));
    let document_xml = docx_document_xml(doc);
    Exported {
        bytes: office_zip(limits, vec![
            (
                "[Content_Types].xml".to_string(),
                br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#.to_vec(),
            ),
            (
                "_rels/.rels".to_string(),
                br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#.to_vec(),
            ),
            ("word/document.xml".to_string(), document_xml.into_bytes()),
        ]),
        loss_report: finish_report(report),
    }
}

pub fn export_pptx(doc: &Document) -> Exported {
    export_pptx_with_limits(doc, ResourceLimits::default())
}

pub fn export_pptx_with_limits(doc: &Document, limits: ResourceLimits) -> Exported {
    let mut report = base_report(doc, "pptx");
    collect_common_losses(&doc.body, "$.body", &mut report);
    report.losses.push(loss(
        "NODX-E026",
        "warning",
        "$",
        "PPTX export maps presentation slides to one slide per `slide` block and flattens unsupported semantics.",
    ));
    let slides = presentation_slides(doc);
    Exported {
        bytes: office_zip(limits, vec![
            (
                "[Content_Types].xml".to_string(),
                pptx_content_types(slides.len()).into_bytes(),
            ),
            (
                "_rels/.rels".to_string(),
                br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/></Relationships>"#.to_vec(),
            ),
            (
                "ppt/presentation.xml".to_string(),
                pptx_presentation_xml(slides.len()).into_bytes(),
            ),
            (
                "ppt/_rels/presentation.xml.rels".to_string(),
                pptx_presentation_rels(slides.len()).into_bytes(),
            ),
        ]
        .into_iter()
        .chain(slides.iter().enumerate().map(|(i, slide)| {
            (
                format!("ppt/slides/slide{}.xml", i + 1),
                pptx_slide_xml(slide).into_bytes(),
            )
        }))
        .collect()),
        loss_report: finish_report(report),
    }
}

pub fn loss_report_json(report: &LossReport) -> String {
    let mut out = String::from("{\"format\":");
    write_json_string(&mut out, &report.format);
    out.push_str(",\"losses\":[");
    for (i, item) in report.losses.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str("{\"code\":");
        write_json_string(&mut out, &item.code);
        out.push_str(",\"message\":");
        write_json_string(&mut out, &item.message);
        out.push_str(",\"path\":");
        write_json_string(&mut out, &item.path);
        out.push_str(",\"severity\":");
        write_json_string(&mut out, &item.severity);
        out.push('}');
    }
    out.push_str("],\"lossy\":");
    out.push_str(if report.lossy { "true" } else { "false" });
    out.push_str(",\"profile\":");
    write_json_string(&mut out, &report.profile);
    out.push_str(",\"schema\":");
    write_json_string(&mut out, &report.schema);
    out.push_str(",\"sourceSchema\":");
    write_json_string(&mut out, &report.source_schema);
    out.push('}');
    out
}

fn base_report(doc: &Document, format: &str) -> LossReport {
    LossReport {
        schema: "nodx/export-loss/1.2".to_string(),
        source_schema: doc
            .meta
            .get("schema")
            .and_then(value_string)
            .unwrap_or(&doc.schema)
            .to_string(),
        profile: PRESENTATION_PROFILE_NAME.to_string(),
        format: format.to_string(),
        lossy: false,
        losses: Vec::new(),
    }
}

fn finish_report(mut report: LossReport) -> LossReport {
    report.lossy = !report.losses.is_empty();
    report
}

fn value_string(value: &Value) -> Option<&str> {
    match value {
        Value::String(s) => Some(s),
        _ => None,
    }
}

fn loss(code: &str, severity: &str, path: &str, message: &str) -> Loss {
    Loss {
        code: code.to_string(),
        severity: severity.to_string(),
        path: path.to_string(),
        message: message.to_string(),
    }
}

fn paged_html_bridge(doc: &Document) -> String {
    let rendered = render_html(doc);
    let insert = "<meta name=\"nodx-export-profile\" content=\"NODX-Presentation-1.2\"><meta name=\"nodx-pdf-bridge\" content=\"safe-paged-html\"><style>@page{size:A4;margin:20mm}.pagebreak{break-before:page}</style>";
    if let Some(pos) = rendered.find("<style>") {
        let mut out = rendered;
        out.insert_str(pos, insert);
        out
    } else {
        rendered
    }
}

fn push_markdown_nodes(
    out: &mut String,
    nodes: &[Node],
    path: &str,
    report: &mut LossReport,
    quote_depth: usize,
) {
    for (i, node) in nodes.iter().enumerate() {
        let node_path = format!("{path}[{i}]");
        match node.node_type.as_str() {
            "heading" => {
                let level = node
                    .attrs
                    .get("level")
                    .and_then(|v| v.parse::<usize>().ok())
                    .unwrap_or(1)
                    .clamp(1, 6);
                push_quote_prefix(out, quote_depth);
                out.push_str(&"#".repeat(level));
                out.push(' ');
                out.push_str(&markdown_inlines(&node.inlines));
                push_markdown_attrs(out, node);
                out.push_str("\n\n");
            }
            "paragraph" | "caption" | "citation-entry" => {
                push_wrapped_markdown_text(out, &markdown_inlines(&node.inlines), quote_depth);
                out.push('\n');
            }
            "list" => {
                let ordered = node.attrs.get("kind").map(String::as_str) == Some("ordered");
                for (idx, item) in node.children.iter().enumerate() {
                    push_quote_prefix(out, quote_depth);
                    let marker = if ordered {
                        format!("{}. ", idx + 1)
                    } else if item.attrs.get("checked").map(String::as_str) == Some("true") {
                        "- [x] ".to_string()
                    } else if item.attrs.get("checked").map(String::as_str) == Some("false") {
                        "- [ ] ".to_string()
                    } else {
                        "- ".to_string()
                    };
                    out.push_str(&marker);
                    out.push_str(&markdown_inlines(&item.inlines));
                    out.push('\n');
                }
                out.push('\n');
            }
            "table" => push_markdown_table(out, node, quote_depth),
            "code" | "pre" => {
                let text = node.text.as_deref().unwrap_or("");
                let fence = markdown_code_fence(text);
                push_quote_prefix(out, quote_depth);
                out.push_str(&fence);
                if let Some(lang) = node
                    .attrs
                    .get("lang")
                    .or_else(|| node.attrs.get("language"))
                {
                    out.push_str(lang);
                }
                out.push('\n');
                for line in text.lines() {
                    push_quote_prefix(out, quote_depth);
                    out.push_str(line);
                    out.push('\n');
                }
                push_quote_prefix(out, quote_depth);
                out.push_str(&fence);
                out.push_str("\n\n");
            }
            "math" => {
                report.losses.push(loss(
                    "NODX-E026",
                    "warning",
                    &node_path,
                    "Block math is exported as a fenced code block.",
                ));
                push_quote_prefix(out, quote_depth);
                out.push_str("```math\n");
                for line in node.text.as_deref().unwrap_or("").lines() {
                    push_quote_prefix(out, quote_depth);
                    out.push_str(line);
                    out.push('\n');
                }
                push_quote_prefix(out, quote_depth);
                out.push_str("```\n\n");
            }
            "image" => {
                let alt = node.attrs.get("alt").map(String::as_str).unwrap_or("");
                let src = node.attrs.get("src").map(String::as_str).unwrap_or("");
                push_quote_prefix(out, quote_depth);
                out.push_str("![");
                out.push_str(&escape_markdown_text(alt));
                out.push_str("](");
                out.push_str(src);
                out.push_str(")\n\n");
            }
            "pagebreak" => {
                push_quote_prefix(out, quote_depth);
                out.push_str("---\n\n");
            }
            "note" | "info" | "tip" | "important" | "caution" | "warning" | "danger"
            | "example" | "summary" | "quote" => {
                if node.node_type != "quote" {
                    push_quote_prefix(out, quote_depth);
                    out.push_str("> **");
                    out.push_str(markdown_callout_label(node));
                    out.push_str("**\n");
                }
                push_markdown_nodes(
                    out,
                    &node.children,
                    &format!("{node_path}.children"),
                    report,
                    quote_depth + 1,
                );
                out.push('\n');
            }
            "style" | "toc" | "media" | "embed" | "include" => {
                report.losses.push(loss(
                    "NODX-E026",
                    "warning",
                    &node_path,
                    &format!(
                        "`{}` is not represented in Markdown and was flattened or omitted.",
                        node.node_type
                    ),
                ));
                if !node.children.is_empty() {
                    push_markdown_nodes(
                        out,
                        &node.children,
                        &format!("{node_path}.children"),
                        report,
                        quote_depth,
                    );
                }
            }
            _ => {
                if !node.inlines.is_empty() {
                    report.losses.push(loss(
                        "NODX-E026",
                        "warning",
                        &node_path,
                        &format!("`{}` was flattened to paragraph Markdown.", node.node_type),
                    ));
                    push_wrapped_markdown_text(out, &markdown_inlines(&node.inlines), quote_depth);
                    out.push('\n');
                }
                if !node.children.is_empty() {
                    report.losses.push(loss(
                        "NODX-E026",
                        "warning",
                        &node_path,
                        &format!("`{}` container semantics were flattened.", node.node_type),
                    ));
                    push_markdown_nodes(
                        out,
                        &node.children,
                        &format!("{node_path}.children"),
                        report,
                        quote_depth,
                    );
                }
            }
        }
    }
}

fn push_markdown_table(out: &mut String, node: &Node, quote_depth: usize) {
    let mut rows: Vec<Vec<String>> = node
        .children
        .iter()
        .filter(|row| row.node_type == "row")
        .map(|row| {
            row.children
                .iter()
                .map(|cell| escape_markdown_cell(&markdown_inlines(&cell.inlines)))
                .collect::<Vec<_>>()
        })
        .collect();
    if rows.is_empty() {
        return;
    }
    let width = rows.iter().map(Vec::len).max().unwrap_or(0);
    for row in &mut rows {
        row.resize(width, String::new());
    }
    push_quote_prefix(out, quote_depth);
    out.push('|');
    for cell in &rows[0] {
        out.push(' ');
        out.push_str(cell);
        out.push_str(" |");
    }
    out.push('\n');
    push_quote_prefix(out, quote_depth);
    out.push('|');
    for _ in 0..width {
        out.push_str(" --- |");
    }
    out.push('\n');
    for row in rows.iter().skip(1) {
        push_quote_prefix(out, quote_depth);
        out.push('|');
        for cell in row {
            out.push(' ');
            out.push_str(cell);
            out.push_str(" |");
        }
        out.push('\n');
    }
    out.push('\n');
}

fn markdown_inlines(inlines: &[Inline]) -> String {
    let mut out = String::new();
    for inline in inlines {
        match inline {
            Inline::Text(text) => out.push_str(&escape_markdown_text(text)),
            Inline::Code(text) => {
                out.push('`');
                out.push_str(&text.replace('`', "\\`"));
                out.push('`');
            }
            Inline::Strong(children) => {
                out.push_str("**");
                out.push_str(&markdown_inlines(children));
                out.push_str("**");
            }
            Inline::Em(children) => {
                out.push('*');
                out.push_str(&markdown_inlines(children));
                out.push('*');
            }
            Inline::Strike(children) => {
                out.push_str("~~");
                out.push_str(&markdown_inlines(children));
                out.push_str("~~");
            }
            Inline::Sub(children) => {
                out.push('~');
                out.push_str(&markdown_inlines(children));
                out.push('~');
            }
            Inline::Sup(children) => {
                out.push('^');
                out.push_str(&markdown_inlines(children));
                out.push('^');
            }
            Inline::Mark { children, .. } | Inline::Span { children, .. } => {
                out.push_str(&markdown_inlines(children));
            }
            Inline::Link { label, target, .. } => {
                out.push('[');
                out.push_str(&markdown_inlines(label));
                out.push_str("](");
                out.push_str(target);
                out.push(')');
            }
            Inline::Var { namespace, name } => {
                if namespace == "vars" {
                    out.push_str("{{");
                    out.push_str(name);
                    out.push_str("}}");
                } else {
                    out.push_str("{{");
                    out.push_str(namespace);
                    out.push('.');
                    out.push_str(name);
                    out.push_str("}}");
                }
            }
            Inline::Ref { target } => {
                out.push_str("@[");
                out.push_str(target);
                out.push(']');
            }
            Inline::Mention { kind, target } => {
                out.push_str("@{");
                out.push_str(kind);
                out.push(':');
                out.push_str(target);
                out.push('}');
            }
            Inline::FootnoteRef { target } => {
                out.push_str("[^");
                out.push_str(target);
                out.push(']');
            }
            Inline::CitationRef { target } => {
                out.push_str("[@");
                out.push_str(target);
                out.push(']');
            }
            Inline::MathInline { source } => {
                out.push_str("$$");
                out.push_str(source);
                out.push_str("$$");
            }
            Inline::LineBreak => {
                // Markdown hard break: trailing backslash. The conversion
                // pipeline reverses the NODX source form `… \\n`.
                out.push_str("\\\n");
            }
        }
    }
    out
}

fn push_markdown_attrs(out: &mut String, node: &Node) {
    if node.id.is_none() && node.classes.is_empty() {
        return;
    }
    out.push_str(" {");
    if let Some(id) = &node.id {
        out.push('#');
        out.push_str(id);
    }
    for class in &node.classes {
        if node.id.is_some() || !out.ends_with('{') {
            out.push(' ');
        }
        out.push('.');
        out.push_str(class);
    }
    out.push('}');
}

fn push_wrapped_markdown_text(out: &mut String, text: &str, quote_depth: usize) {
    for line in text.lines() {
        push_quote_prefix(out, quote_depth);
        out.push_str(line);
        out.push('\n');
    }
}

fn push_quote_prefix(out: &mut String, quote_depth: usize) {
    for _ in 0..quote_depth {
        out.push_str("> ");
    }
}

fn markdown_callout_label(node: &Node) -> &str {
    match node.node_type.as_str() {
        "note" => node.attrs.get("type").map(String::as_str).unwrap_or("Note"),
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
}

fn escape_markdown_text(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        if matches!(ch, '\\' | '`' | '[' | ']' | '<' | '>') {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

fn escape_markdown_cell(text: &str) -> String {
    text.replace('|', "\\|").replace('\n', "<br>")
}

fn markdown_code_fence(text: &str) -> String {
    let mut max_run = 0usize;
    let mut current = 0usize;
    for ch in text.chars() {
        if ch == '`' {
            current += 1;
            max_run = max_run.max(current);
        } else {
            current = 0;
        }
    }
    "`".repeat(max_run.max(2) + 1)
}

fn convert_markdown_body(source: &str, report: &mut LossReport) -> String {
    let normalized = source.replace("\r\n", "\n");
    let mut out = String::new();
    let body = if has_front_matter(&normalized) {
        normalized.as_str()
    } else {
        out.push_str("---\nschema: nodx/1.0\nprofiles:\n  requires:\n    - core\n---\n\n");
        normalized.as_str()
    };
    let lines: Vec<&str> = body.split('\n').collect();
    let mut i = 0usize;
    let mut in_front_matter = false;
    while i < lines.len() {
        let line = lines[i];
        if i == 0 && line == "---" {
            in_front_matter = true;
            out.push_str(line);
            out.push('\n');
            i += 1;
            continue;
        }
        if in_front_matter {
            out.push_str(line);
            out.push('\n');
            if line == "---" {
                in_front_matter = false;
            }
            i += 1;
            continue;
        }
        if let Some((fence, info)) = markdown_fence(line) {
            let lang = info.split_whitespace().next().unwrap_or("");
            i += 1;
            let start = i;
            while i < lines.len() && !lines[i].starts_with(fence) {
                i += 1;
            }
            let literal_lines = &lines[start..i];
            if i < lines.len() {
                i += 1;
            }
            let colons = nodx_literal_fence(literal_lines);
            out.push_str(&colons);
            out.push_str("code");
            if !lang.is_empty() {
                out.push_str(" {lang=\"");
                out.push_str(&nodx_attr_value(lang));
                out.push_str("\"}");
            }
            out.push('\n');
            for literal in literal_lines {
                out.push_str(literal);
                out.push('\n');
            }
            out.push_str(&colons);
            out.push('\n');
            continue;
        }
        if let Some((alt, src)) = markdown_image_line(line) {
            out.push_str(":::image {src=\"");
            out.push_str(&nodx_attr_value(src));
            out.push_str("\" alt=\"");
            out.push_str(&nodx_attr_value(alt));
            out.push_str("\"}\n:::\n");
            i += 1;
            continue;
        }
        if line.trim_start().starts_with('<') {
            report.losses.push(loss(
                "NODX-E026",
                "warning",
                &format!("$.lines[{i}]"),
                "Raw HTML is preserved as text; NODX renderers will escape it.",
            ));
        }
        let line = normalize_markdown_links(line, report, i);
        out.push_str(&line);
        if i + 1 < lines.len() {
            out.push('\n');
        }
        i += 1;
    }
    out
}

fn has_front_matter(source: &str) -> bool {
    if !source.starts_with("---\n") {
        return false;
    }
    source.lines().skip(1).any(|line| line == "---")
}

fn markdown_fence(line: &str) -> Option<(&'static str, &str)> {
    if let Some(rest) = line.strip_prefix("```") {
        Some(("```", rest.trim()))
    } else {
        line.strip_prefix("~~~").map(|rest| ("~~~", rest.trim()))
    }
}

fn nodx_literal_fence(lines: &[&str]) -> String {
    let mut max_run = 0usize;
    for line in lines {
        let run = line.chars().take_while(|ch| *ch == ':').count();
        if run > max_run {
            max_run = run;
        }
    }
    ":".repeat(max_run.max(2) + 1)
}

fn markdown_image_line(line: &str) -> Option<(&str, &str)> {
    let trimmed = line.trim();
    let rest = trimmed.strip_prefix("![")?;
    let close = rest.find("](")?;
    if !rest.ends_with(')') {
        return None;
    }
    let alt = &rest[..close];
    let src = &rest[close + 2..rest.len() - 1];
    if src.is_empty() {
        return None;
    }
    Some((alt, src))
}

fn nodx_attr_value(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch == '"' { '\'' } else { ch })
        .collect()
}

fn normalize_markdown_links(line: &str, report: &mut LossReport, line_no: usize) -> String {
    let mut out = String::new();
    let mut rest = line;
    while let Some(label_start) = rest.find('[') {
        let (before, after_start) = rest.split_at(label_start);
        out.push_str(before);
        if after_start.starts_with("![") {
            out.push('!');
            rest = &after_start[1..];
            continue;
        }
        let Some(label_end) = after_start.find("](") else {
            out.push_str(after_start);
            return out;
        };
        let target_start = label_end + 2;
        let Some(target_end_rel) = after_start[target_start..].find(')') else {
            out.push_str(after_start);
            return out;
        };
        let target_end = target_start + target_end_rel;
        let target = &after_start[target_start..target_end];
        out.push_str(&after_start[..target_start]);
        out.push_str(&normalize_markdown_target(target, report, line_no));
        out.push(')');
        rest = &after_start[target_end + 1..];
    }
    out.push_str(rest);
    out
}

fn normalize_markdown_target(target: &str, report: &mut LossReport, line_no: usize) -> String {
    if target.starts_with('#') || target.contains(':') {
        return target.to_string();
    }
    let mut normalized = target;
    let mut changed = false;
    while let Some(rest) = normalized.strip_prefix("./") {
        normalized = rest;
        changed = true;
    }
    while let Some(rest) = normalized.strip_prefix("../") {
        normalized = rest;
        changed = true;
    }
    while let Some(rest) = normalized.strip_suffix('/') {
        normalized = rest;
        changed = true;
    }
    if changed {
        report.losses.push(loss(
            "NODX-E026",
            "warning",
            &format!("$.lines[{line_no}]"),
            "Relative Markdown link was normalized to a NODX package-safe path.",
        ));
    }
    normalized.to_string()
}

fn collect_common_losses(nodes: &[Node], path: &str, report: &mut LossReport) {
    for (i, node) in nodes.iter().enumerate() {
        let node_path = format!("{path}[{i}]");
        match node.node_type.as_str() {
            "style" => report.losses.push(loss(
                "NODX-E026",
                "warning",
                &node_path,
                "Style rules are not faithfully mapped by this exporter.",
            )),
            "math" => report.losses.push(loss(
                "NODX-E026",
                "warning",
                &node_path,
                "Math is exported as source text.",
            )),
            "image" | "media" | "embed" => report.losses.push(loss(
                "NODX-E026",
                "warning",
                &node_path,
                "External media is replaced by textual fallback.",
            )),
            "toc" => report.losses.push(loss(
                "NODX-E026",
                "warning",
                &node_path,
                "Generated navigation is flattened.",
            )),
            "speaker-notes" if report.format == "docx" => report.losses.push(loss(
                "NODX-E026",
                "warning",
                &node_path,
                "Speaker notes are rendered as normal document text in DOCX.",
            )),
            _ => {}
        }
        collect_inline_losses(&node.inlines, &format!("{node_path}.inlines"), report);
        collect_common_losses(&node.children, &format!("{node_path}.children"), report);
    }
}

fn collect_inline_losses(inlines: &[Inline], path: &str, report: &mut LossReport) {
    for (i, item) in inlines.iter().enumerate() {
        let inline_path = format!("{path}[{i}]");
        match item {
            Inline::Link { label, .. } => {
                report.losses.push(loss(
                    "NODX-E026",
                    "warning",
                    &inline_path,
                    "Link target is preserved as visible text only.",
                ));
                collect_inline_losses(label, &format!("{inline_path}.label"), report);
            }
            Inline::Var { .. } | Inline::Ref { .. } | Inline::Mention { .. } => {
                report.losses.push(loss(
                    "NODX-E026",
                    "warning",
                    &inline_path,
                    "Agent-readable inline semantics are flattened to text.",
                ));
            }
            Inline::MathInline { .. } => report.losses.push(loss(
                "NODX-E026",
                "warning",
                &inline_path,
                "Inline math is exported as source text.",
            )),
            Inline::Strong(children)
            | Inline::Em(children)
            | Inline::Strike(children)
            | Inline::Sub(children)
            | Inline::Sup(children) => {
                collect_inline_losses(children, &format!("{inline_path}.children"), report)
            }
            Inline::Mark { children, .. } => {
                collect_inline_losses(children, &format!("{inline_path}.children"), report)
            }
            Inline::Span { children, .. } => {
                report.losses.push(loss(
                    "NODX-E026",
                    "warning",
                    &inline_path,
                    "Span attributes are flattened.",
                ));
                collect_inline_losses(children, &format!("{inline_path}.children"), report);
            }
            Inline::Text(_)
            | Inline::Code(_)
            | Inline::FootnoteRef { .. }
            | Inline::CitationRef { .. }
            | Inline::LineBreak => {}
        }
    }
}

fn docx_document_xml(doc: &Document) -> String {
    let mut out = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>"#,
    );
    for node in &doc.body {
        push_docx_node(&mut out, node);
    }
    out.push_str("<w:sectPr/></w:body></w:document>");
    out
}

fn push_docx_node(out: &mut String, node: &Node) {
    match node.node_type.as_str() {
        "heading" => push_docx_paragraph(out, &node.inlines),
        "paragraph" | "item" | "caption" | "citation-entry" => {
            push_docx_paragraph(out, &node.inlines)
        }
        "code" | "pre" | "math" => {
            push_docx_text_paragraph(out, node.text.as_deref().unwrap_or(""))
        }
        "image" | "media" | "embed" => push_docx_text_paragraph(
            out,
            node.attrs
                .get("alt")
                .map(String::as_str)
                .unwrap_or(&node.node_type),
        ),
        "pagebreak" => out.push_str(r#"<w:p><w:r><w:br w:type="page"/></w:r></w:p>"#),
        _ => {
            if !node.inlines.is_empty() {
                push_docx_paragraph(out, &node.inlines);
            }
            for child in &node.children {
                push_docx_node(out, child);
            }
        }
    }
}

fn push_docx_paragraph(out: &mut String, inlines: &[Inline]) {
    let text = inline_text(inlines);
    push_docx_text_paragraph(out, &text);
}

fn push_docx_text_paragraph(out: &mut String, text: &str) {
    out.push_str("<w:p><w:r><w:t>");
    escape_xml(out, text);
    out.push_str("</w:t></w:r></w:p>");
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Slide {
    title: String,
    lines: Vec<String>,
}

fn presentation_slides(doc: &Document) -> Vec<Slide> {
    let mut slides = Vec::new();
    collect_slides(&doc.body, &mut slides);
    if slides.is_empty() {
        slides.push(Slide {
            title: doc
                .meta
                .get("title")
                .and_then(value_string)
                .unwrap_or("Untitled")
                .to_string(),
            lines: text_lines(&doc.body),
        });
    }
    slides
}

fn collect_slides(nodes: &[Node], slides: &mut Vec<Slide>) {
    for node in nodes {
        if node.node_type == "slide" {
            slides.push(slide_from_node(node));
        } else {
            collect_slides(&node.children, slides);
        }
    }
}

fn slide_from_node(node: &Node) -> Slide {
    let mut title = node
        .attrs
        .get("title")
        .cloned()
        .unwrap_or_else(|| "Slide".to_string());
    for child in &node.children {
        if child.node_type == "heading" {
            title = inline_text(&child.inlines);
            break;
        }
    }
    Slide {
        title,
        lines: text_lines(&node.children),
    }
}

fn text_lines(nodes: &[Node]) -> Vec<String> {
    let mut lines = Vec::new();
    for node in nodes {
        match node.node_type.as_str() {
            "heading" | "paragraph" | "item" | "caption" | "citation-entry" => {
                let text = inline_text(&node.inlines);
                if !text.is_empty() {
                    lines.push(text);
                }
            }
            "code" | "pre" | "math" => {
                if let Some(text) = &node.text {
                    lines.push(text.clone());
                }
            }
            "speaker-notes" => {}
            "image" | "media" | "embed" => {
                if let Some(alt) = node.attrs.get("alt") {
                    lines.push(alt.clone());
                }
            }
            _ => lines.extend(text_lines(&node.children)),
        }
    }
    lines
}

fn inline_text(inlines: &[Inline]) -> String {
    let mut out = String::new();
    for inline in inlines {
        match inline {
            Inline::Text(text) | Inline::Code(text) => out.push_str(text),
            Inline::Strong(children)
            | Inline::Em(children)
            | Inline::Strike(children)
            | Inline::Sub(children)
            | Inline::Sup(children) => out.push_str(&inline_text(children)),
            Inline::Mark { children, .. } => out.push_str(&inline_text(children)),
            Inline::Link { label, target, .. } => {
                out.push_str(&inline_text(label));
                out.push_str(" (");
                out.push_str(target);
                out.push(')');
            }
            Inline::Span { children, .. } => out.push_str(&inline_text(children)),
            Inline::Var { namespace, name } => {
                out.push_str(namespace);
                out.push('.');
                out.push_str(name);
            }
            Inline::Ref { target }
            | Inline::Mention { target, .. }
            | Inline::FootnoteRef { target }
            | Inline::CitationRef { target } => out.push_str(target),
            Inline::MathInline { source } => out.push_str(source),
            Inline::LineBreak => out.push(' '),
        }
    }
    out
}

fn pptx_content_types(slide_count: usize) -> String {
    let mut out = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>"#,
    );
    for i in 1..=slide_count {
        out.push_str(&format!(r#"<Override PartName="/ppt/slides/slide{i}.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>"#));
    }
    out.push_str("</Types>");
    out
}

fn pptx_presentation_xml(slide_count: usize) -> String {
    let mut out = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><p:sldIdLst>"#,
    );
    for i in 1..=slide_count {
        out.push_str(&format!(r#"<p:sldId id="{}" r:id="rId{}"/>"#, 255 + i, i));
    }
    out.push_str("</p:sldIdLst></p:presentation>");
    out
}

fn pptx_presentation_rels(slide_count: usize) -> String {
    let mut out = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
    );
    for i in 1..=slide_count {
        out.push_str(&format!(r#"<Relationship Id="rId{i}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide{i}.xml"/>"#));
    }
    out.push_str("</Relationships>");
    out
}

fn pptx_slide_xml(slide: &Slide) -> String {
    let mut out = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/><p:sp><p:nvSpPr><p:cNvPr id="2" name="Title"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>"#,
    );
    escape_xml(&mut out, &slide.title);
    out.push_str("</a:t></a:r></a:p></p:txBody></p:sp><p:sp><p:nvSpPr><p:cNvPr id=\"3\" name=\"Body\"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:txBody><a:bodyPr/><a:lstStyle/>");
    for line in &slide.lines {
        out.push_str("<a:p><a:r><a:t>");
        escape_xml(&mut out, line);
        out.push_str("</a:t></a:r></a:p>");
    }
    out.push_str("</p:txBody></p:sp></p:spTree></p:cSld></p:sld>");
    out
}

fn office_zip(limits: ResourceLimits, entries: Vec<(String, Vec<u8>)>) -> Vec<u8> {
    if entries.len() > limits.export_entry_count {
        return Vec::new();
    }
    let total: usize = entries.iter().map(|(_, d)| d.len()).sum();
    if total > limits.export_bytes {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, data) in entries {
        let local_offset = out.len() as u32;
        let crc = core_crc32(&data);
        write_u32(&mut out, 0x0403_4b50);
        write_u16(&mut out, 20);
        write_u16(&mut out, 0);
        write_u16(&mut out, 0);
        write_u16(&mut out, 0);
        write_u16(&mut out, 0);
        write_u32(&mut out, crc);
        write_u32(&mut out, data.len() as u32);
        write_u32(&mut out, data.len() as u32);
        write_u16(&mut out, name.len() as u16);
        write_u16(&mut out, 0);
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(&data);

        write_u32(&mut central, 0x0201_4b50);
        write_u16(&mut central, 20);
        write_u16(&mut central, 20);
        write_u16(&mut central, 0);
        write_u16(&mut central, 0);
        write_u16(&mut central, 0);
        write_u16(&mut central, 0);
        write_u32(&mut central, crc);
        write_u32(&mut central, data.len() as u32);
        write_u32(&mut central, data.len() as u32);
        write_u16(&mut central, name.len() as u16);
        write_u16(&mut central, 0);
        write_u16(&mut central, 0);
        write_u16(&mut central, 0);
        write_u16(&mut central, 0);
        write_u32(&mut central, 0);
        write_u32(&mut central, local_offset);
        central.extend_from_slice(name.as_bytes());
    }
    let central_offset = out.len() as u32;
    let central_size = central.len() as u32;
    let entry_count = count_central_entries(&central);
    out.extend_from_slice(&central);
    write_u32(&mut out, 0x0605_4b50);
    write_u16(&mut out, 0);
    write_u16(&mut out, 0);
    write_u16(&mut out, entry_count);
    write_u16(&mut out, entry_count);
    write_u32(&mut out, central_size);
    write_u32(&mut out, central_offset);
    write_u16(&mut out, 0);
    out
}

fn count_central_entries(central: &[u8]) -> u16 {
    central
        .windows(4)
        .filter(|window| *window == [0x50, 0x4b, 0x01, 0x02])
        .count() as u16
}

fn write_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn escape_xml(out: &mut String, input: &str) {
    for ch in input.chars() {
        // XML 1.0 only allows tab, LF, CR, and characters >= 0x20 (plus broader
        // unicode planes). Strip anything else so external validators do not
        // reject the output.
        let code = ch as u32;
        if code < 0x20 && ch != '\t' && ch != '\n' && ch != '\r' {
            continue;
        }
        if code == 0xfffe || code == 0xffff {
            continue;
        }
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
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
            c if c < ' ' => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use nodx_core::parse_str;

    use super::*;

    #[test]
    fn presentation_profile_is_named() {
        assert_eq!(presentation_profile(), "NODX-Presentation-1.2");
    }

    #[test]
    fn pdf_export_is_safe_html_bridge_with_loss_report() {
        let doc = parse_str("# Deck\n");
        let exported = export_pdf_bridge(&doc);
        let html = String::from_utf8(exported.bytes).unwrap();
        assert!(html.contains("nodx-pdf-bridge"));
        assert_eq!(exported.loss_report.format, "pdf");
        assert!(exported.loss_report.lossy);
    }

    #[test]
    fn docx_export_emits_zip_and_loss_report() {
        let doc = parse_str("# Title\n\n:::style\nh1 { color: red; }\n:::\n");
        let exported = export_docx(&doc);
        assert!(exported.bytes.starts_with(b"PK\x03\x04"));
        assert!(loss_report_json(&exported.loss_report).contains("\"format\":\"docx\""));
        assert!(
            exported
                .loss_report
                .losses
                .iter()
                .any(|loss| loss.message.contains("Style rules"))
        );
    }

    #[test]
    fn pptx_export_uses_slide_fixtures() {
        let doc = parse_str(":::slide {title=\"One\"}\n# First\nBody\n:::\n");
        let exported = export_pptx(&doc);
        assert!(exported.bytes.starts_with(b"PK\x03\x04"));
        assert!(loss_report_json(&exported.loss_report).contains("\"format\":\"pptx\""));
    }
}

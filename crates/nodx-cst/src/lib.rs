#![forbid(unsafe_code)]

use std::ops::Range;

use nodx_agent_sdk::{Batch, MutationError, Operation, Target, apply_batch};
use nodx_core::{Document, Node, ResourceLimits, parse_str_with_limits, valid_name};
use nodx_validate::{Validator, exit_code_for};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CstDocument {
    source: Vec<u8>,
    ast: Document,
    nodes: Vec<CstNode>,
    diagnostics: Vec<CstDiagnostic>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CstNode {
    pub kind: String,
    pub ast_path: Vec<usize>,
    pub byte_range: Range<usize>,
    pub opener_range: Option<Range<usize>>,
    pub attr_range: Option<Range<usize>>,
    attr_insert: Option<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CstDiagnostic {
    pub message: String,
    pub byte_offset: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Patch {
    pub range: Range<usize>,
    pub replacement: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PatchSet {
    patches: Vec<Patch>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CstError {
    InvalidUtf8,
    InvalidPatchRange,
    OverlappingPatches,
    MissingNode,
    UnsupportedPatch,
    InvalidAttribute,
    ValidationFailed,
    SourceTooLarge,
    Mutation(MutationError),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Line {
    start: usize,
    content_end: usize,
    end: usize,
}

pub fn parse_bytes(source: &[u8]) -> Result<CstDocument, CstError> {
    parse_bytes_with_limits(source, ResourceLimits::default())
}

pub fn parse_bytes_with_limits(
    source: &[u8],
    limits: ResourceLimits,
) -> Result<CstDocument, CstError> {
    if source.len() > limits.source_bytes {
        return Err(CstError::SourceTooLarge);
    }
    let text = std::str::from_utf8(source).map_err(|_| CstError::InvalidUtf8)?;
    let ast = parse_str_with_limits(text, limits);
    let lines = lines(source);
    let mut diagnostics = Vec::new();
    let mut nodes = Vec::new();
    let start_line = front_matter_end(text, &lines).unwrap_or(0);
    scan_blocks(
        text,
        &lines,
        start_line,
        lines.len(),
        &mut Vec::new(),
        None,
        &mut nodes,
        &mut diagnostics,
    );
    Ok(CstDocument {
        source: source.to_vec(),
        ast,
        nodes,
        diagnostics,
    })
}

pub fn parse_str_lossless(source: &str) -> CstDocument {
    parse_bytes(source.as_bytes()).expect("str input is valid UTF-8")
}

impl CstDocument {
    pub fn ast(&self) -> &Document {
        &self.ast
    }

    pub fn nodes(&self) -> &[CstNode] {
        &self.nodes
    }

    pub fn diagnostics(&self) -> &[CstDiagnostic] {
        &self.diagnostics
    }

    pub fn emit(&self) -> &[u8] {
        &self.source
    }

    pub fn ast_node(&self, path: &[usize]) -> Option<&Node> {
        ast_node(&self.ast.body, path)
    }

    pub fn cst_node(&self, path: &[usize]) -> Option<&CstNode> {
        self.nodes.iter().find(|node| node.ast_path == path)
    }

    pub fn apply_patches(&self, patches: &PatchSet) -> Result<CstDocument, CstError> {
        let mut out = self.source.clone();
        let mut sorted = patches.patches.clone();
        sorted.sort_by_key(|patch| patch.range.start);
        let mut previous_end = 0;
        for patch in &sorted {
            if patch.range.start > patch.range.end || patch.range.end > self.source.len() {
                return Err(CstError::InvalidPatchRange);
            }
            if patch.range.start < previous_end {
                return Err(CstError::OverlappingPatches);
            }
            previous_end = patch.range.end;
        }
        for patch in sorted.iter().rev() {
            out.splice(patch.range.clone(), patch.replacement.clone());
        }
        parse_bytes(&out)
    }

    pub fn set_named_attribute(
        &self,
        path: &[usize],
        name: &str,
        value: &str,
    ) -> Result<PatchSet, CstError> {
        if name == "id" || name == "class" || name == "classes" || !valid_name(name, true) {
            return Err(CstError::InvalidAttribute);
        }
        let node = self.cst_node(path).ok_or(CstError::MissingNode)?;
        let replacement = quote_attr_value(value).into_bytes();
        if let Some(attr_range) = &node.attr_range {
            let text = std::str::from_utf8(&self.source[attr_range.clone()])
                .map_err(|_| CstError::InvalidUtf8)?;
            if let Some(value_range) = find_attr_value(text, attr_range.start, name) {
                return Ok(PatchSet::single(value_range, replacement));
            }
            let insert_at = attr_range.end - 1;
            let prefix = if attr_range.end - attr_range.start <= 2 {
                ""
            } else {
                " "
            };
            return Ok(PatchSet::single(
                insert_at..insert_at,
                format!("{prefix}{name}={}", quote_attr_value(value)).into_bytes(),
            ));
        }
        let insert_at = node.attr_insert.ok_or(CstError::UnsupportedPatch)?;
        Ok(PatchSet::single(
            insert_at..insert_at,
            format!(" {{{name}={}}}", quote_attr_value(value)).into_bytes(),
        ))
    }

    pub fn apply_agent_batch_minimal(
        &self,
        batch: &Batch,
    ) -> Result<(CstDocument, nodx_agent_sdk::MutationReport), CstError> {
        let mut validated = self.ast.clone();
        let report = apply_batch(&mut validated, batch).map_err(CstError::Mutation)?;
        let patched = self.apply_agent_patches(batch)?;
        if patched.ast != validated {
            eprintln!("PATCHED META: {:#?}", patched.ast.meta);
            eprintln!("VALIDATED META: {:#?}", validated.meta);
            return Err(CstError::ValidationFailed);
        }
        let diagnostics = Validator::default().validate(&patched.ast);
        if exit_code_for(&diagnostics) != 0 {
            return Err(CstError::ValidationFailed);
        }
        Ok((patched, report))
    }

    fn apply_agent_patches(&self, batch: &Batch) -> Result<CstDocument, CstError> {
        let mut working = self.clone();
        for operation in &batch.operations {
            match operation {
                Operation::AddAttribute {
                    target,
                    name,
                    value,
                    ..
                }
                | Operation::SetAttribute {
                    target,
                    name,
                    value,
                    ..
                } => {
                    let path = resolve_target_path(&working.ast, target)?;
                    let patch = working.set_named_attribute(&path, name, value)?;
                    working = working.apply_patches(&patch)?;
                }
                Operation::RemoveAttribute { target, name, .. } => {
                    let path = resolve_target_path(&working.ast, target)?;
                    let patch = working.remove_named_attribute(&path, name)?;
                    working = working.apply_patches(&patch)?;
                }
                Operation::Approve {
                    target, reviewer, ..
                } => {
                    let path = resolve_target_path(&working.ast, target)?;
                    let mut patches = PatchSet::new();
                    patches.extend(working.set_named_attribute(&path, "status", "approved")?);
                    if let Some(reviewer) = reviewer {
                        patches.extend(
                            working.set_named_attribute(&path, "reviewed-by", reviewer)?,
                        );
                    }
                    working = working.apply_patches(&patches)?;
                }
                Operation::Reject {
                    target, reviewer, ..
                } => {
                    let path = resolve_target_path(&working.ast, target)?;
                    let mut patches = PatchSet::new();
                    patches.extend(working.set_named_attribute(&path, "status", "rejected")?);
                    if let Some(reviewer) = reviewer {
                        patches.extend(
                            working.set_named_attribute(&path, "reviewed-by", reviewer)?,
                        );
                    }
                    working = working.apply_patches(&patches)?;
                }
                Operation::Delete { target, .. } => {
                    let path = resolve_target_path(&working.ast, target)?;
                    let patch = working.delete_node_range(&path)?;
                    working = working.apply_patches(&patch)?;
                }
                Operation::AddComment {
                    target,
                    author,
                    text,
                    ..
                } => {
                    let path = resolve_target_path(&working.ast, target)?;
                    let patch = working.insert_comment_after(&path, author.as_deref(), text)?;
                    working = working.apply_patches(&patch)?;
                }
                Operation::Replace { target, node, .. } => {
                    let path = resolve_target_path(&working.ast, target)?;
                    let patch = working.replace_node_range(&path, node)?;
                    working = working.apply_patches(&patch)?;
                }
                Operation::Insert {
                    target,
                    position,
                    node,
                    ..
                } => {
                    let path = resolve_target_path(&working.ast, target)?;
                    let patch = working.insert_node(&path, position, node)?;
                    working = working.apply_patches(&patch)?;
                }
            }
        }
        Ok(working)
    }

    fn insert_comment_after(
        &self,
        path: &[usize],
        author: Option<&str>,
        text: &str,
    ) -> Result<PatchSet, CstError> {
        let node = self.cst_node(path).ok_or(CstError::MissingNode)?;
        let mut serialized = String::new();
        if !self.source.is_empty()
            && node.byte_range.end < self.source.len()
            && self.source.get(node.byte_range.end) != Some(&b'\n')
        {
            serialized.push('\n');
        }
        serialized.push_str(":::comment");
        let mut attrs = String::new();
        if let Some(author) = author {
            attrs.push_str(&format!(" {{author={}}}", quote_attr_value(author)));
        }
        if !attrs.is_empty() {
            serialized.push_str(&attrs);
        }
        serialized.push('\n');
        for line in text.lines() {
            serialized.push_str(line);
            serialized.push('\n');
        }
        if !text.ends_with('\n') {
            serialized.push('\n');
        }
        serialized.push_str(":::\n");
        Ok(PatchSet::single(
            node.byte_range.end..node.byte_range.end,
            serialized.into_bytes(),
        ))
    }

    fn replace_node_range(&self, path: &[usize], node: &Node) -> Result<PatchSet, CstError> {
        let cst = self.cst_node(path).ok_or(CstError::MissingNode)?;
        let bytes = serialize_block_node(node, "")?;
        let mut end = cst.byte_range.end;
        if end < self.source.len() && self.source.get(end) == Some(&b'\n') {
            end += 1;
        }
        Ok(PatchSet::single(
            cst.byte_range.start..end,
            bytes.into_bytes(),
        ))
    }

    fn insert_node(
        &self,
        path: &[usize],
        position: &nodx_agent_sdk::InsertPosition,
        node: &Node,
    ) -> Result<PatchSet, CstError> {
        let cst = self.cst_node(path).ok_or(CstError::MissingNode)?;
        let serialized = serialize_block_node(node, "")?;
        let (offset, leading_newline) = match position {
            nodx_agent_sdk::InsertPosition::Before => (cst.byte_range.start, false),
            nodx_agent_sdk::InsertPosition::After => {
                let mut end = cst.byte_range.end;
                if end < self.source.len() && self.source.get(end) == Some(&b'\n') {
                    end += 1;
                }
                (end, false)
            }
            nodx_agent_sdk::InsertPosition::AppendChild => {
                // 1.0 minimal CST does not model container interior offsets reliably;
                // fall back to inserting after the container, signalling that this
                // path is conservative.
                return Err(CstError::UnsupportedPatch);
            }
        };
        let mut bytes = String::new();
        if leading_newline {
            bytes.push('\n');
        }
        bytes.push_str(&serialized);
        Ok(PatchSet::single(offset..offset, bytes.into_bytes()))
    }

    fn delete_node_range(&self, path: &[usize]) -> Result<PatchSet, CstError> {
        let node = self.cst_node(path).ok_or(CstError::MissingNode)?;
        // Drop the node together with its trailing newline so the source stays
        // well-formed.
        let mut end = node.byte_range.end;
        if end < self.source.len() && self.source.get(end) == Some(&b'\n') {
            end += 1;
        }
        Ok(PatchSet::single(node.byte_range.start..end, Vec::new()))
    }

    fn remove_named_attribute(&self, path: &[usize], name: &str) -> Result<PatchSet, CstError> {
        let node = self.cst_node(path).ok_or(CstError::MissingNode)?;
        let attr_range = node.attr_range.as_ref().ok_or(CstError::MissingNode)?;
        let text = std::str::from_utf8(&self.source[attr_range.clone()])
            .map_err(|_| CstError::InvalidUtf8)?;
        let token_range =
            find_attr_token(text, attr_range.start, name).ok_or(CstError::MissingNode)?;
        let mut start = token_range.start;
        let mut end = token_range.end;
        if start > attr_range.start + 1 && self.source.get(start - 1) == Some(&b' ') {
            start -= 1;
        } else if end < attr_range.end - 1 && self.source.get(end) == Some(&b' ') {
            end += 1;
        }
        Ok(PatchSet::single(start..end, Vec::new()))
    }
}

fn serialize_block_node(node: &Node, indent: &str) -> Result<String, CstError> {
    let mut out = String::new();
    match node.node_type.as_str() {
        "paragraph" => {
            let mut buf = String::new();
            serialize_inlines(&node.inlines, &mut buf);
            if buf.is_empty() {
                return Err(CstError::UnsupportedPatch);
            }
            out.push_str(indent);
            out.push_str(&buf);
            out.push('\n');
            out.push('\n');
        }
        "heading" => {
            let level = node
                .attrs
                .get("level")
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(1)
                .clamp(1, 6);
            out.push_str(indent);
            for _ in 0..level {
                out.push('#');
            }
            out.push(' ');
            serialize_inlines(&node.inlines, &mut out);
            serialize_id_attrs(node, &mut out)?;
            out.push('\n');
            out.push('\n');
        }
        "comment" | "note" | "quote" | "section" | "media" | "embed" => {
            out.push_str(indent);
            out.push_str(":::");
            out.push_str(&node.node_type);
            serialize_id_attrs(node, &mut out)?;
            out.push('\n');
            if !node.inlines.is_empty() {
                let mut buf = String::new();
                serialize_inlines(&node.inlines, &mut buf);
                out.push_str(indent);
                out.push_str(&buf);
                out.push('\n');
            }
            for child in &node.children {
                out.push_str(&serialize_block_node(child, indent)?);
            }
            out.push_str(indent);
            out.push_str(":::\n");
        }
        _ => return Err(CstError::UnsupportedPatch),
    }
    Ok(out)
}

fn serialize_id_attrs(node: &Node, out: &mut String) -> Result<(), CstError> {
    let mut attrs = String::new();
    if let Some(id) = &node.id {
        attrs.push('#');
        attrs.push_str(id);
    }
    for (k, v) in &node.attrs {
        if k == "level" {
            continue;
        }
        if !attrs.is_empty() {
            attrs.push(' ');
        }
        attrs.push_str(k);
        attrs.push('=');
        attrs.push_str(&quote_attr_value(v));
    }
    if !attrs.is_empty() {
        out.push_str(" {");
        out.push_str(&attrs);
        out.push('}');
    }
    Ok(())
}

fn serialize_inlines(inlines: &[nodx_core::Inline], out: &mut String) {
    use nodx_core::Inline;
    for inline in inlines {
        match inline {
            Inline::Text(text) => out.push_str(text),
            Inline::Code(text) => {
                out.push('`');
                out.push_str(text);
                out.push('`');
            }
            Inline::Strong(children) => {
                out.push_str("**");
                serialize_inlines(children, out);
                out.push_str("**");
            }
            Inline::Em(children) => {
                out.push('*');
                serialize_inlines(children, out);
                out.push('*');
            }
            Inline::Mark(children) => {
                out.push_str("==");
                serialize_inlines(children, out);
                out.push_str("==");
            }
            Inline::Sub(children) => {
                out.push('~');
                serialize_inlines(children, out);
                out.push('~');
            }
            Inline::Sup(children) => {
                out.push('^');
                serialize_inlines(children, out);
                out.push('^');
            }
            Inline::Link { label, target } => {
                out.push('[');
                serialize_inlines(label, out);
                out.push_str("](");
                out.push_str(target);
                out.push(')');
            }
            Inline::Span { children, .. } => serialize_inlines(children, out),
            Inline::Var { namespace, name } => {
                out.push_str("{{");
                out.push_str(namespace);
                out.push('.');
                out.push_str(name);
                out.push_str("}}");
            }
            Inline::Ref { target } => {
                out.push_str("@[");
                out.push_str(target);
                out.push(']');
            }
            Inline::Mention { kind, target } => {
                out.push('@');
                out.push_str(kind);
                out.push(':');
                out.push_str(target);
            }
            Inline::FootnoteRef { target } | Inline::CitationRef { target } => {
                out.push_str("@[");
                out.push_str(target);
                out.push(']');
            }
            Inline::MathInline { source } => {
                out.push('$');
                out.push_str(source);
                out.push('$');
            }
        }
    }
}

impl PatchSet {
    pub fn new() -> Self {
        Self {
            patches: Vec::new(),
        }
    }

    pub fn single(range: Range<usize>, replacement: Vec<u8>) -> Self {
        Self {
            patches: vec![Patch { range, replacement }],
        }
    }

    pub fn replace(range: Range<usize>, replacement: impl Into<Vec<u8>>) -> Self {
        Self::single(range, replacement.into())
    }

    pub fn insert(offset: usize, bytes: impl Into<Vec<u8>>) -> Self {
        Self::single(offset..offset, bytes.into())
    }

    pub fn delete(range: Range<usize>) -> Self {
        Self::single(range, Vec::new())
    }

    pub fn patches(&self) -> &[Patch] {
        &self.patches
    }

    pub fn extend(&mut self, other: PatchSet) {
        self.patches.extend(other.patches);
    }
}

impl Default for PatchSet {
    fn default() -> Self {
        Self::new()
    }
}

fn lines(source: &[u8]) -> Vec<Line> {
    let mut out = Vec::new();
    let mut start = 0;
    for (index, byte) in source.iter().enumerate() {
        if *byte == b'\n' {
            let content_end = if index > start && source[index - 1] == b'\r' {
                index - 1
            } else {
                index
            };
            out.push(Line {
                start,
                content_end,
                end: index + 1,
            });
            start = index + 1;
        }
    }
    if start < source.len() {
        out.push(Line {
            start,
            content_end: source.len(),
            end: source.len(),
        });
    }
    out
}

fn front_matter_end(text: &str, lines: &[Line]) -> Option<usize> {
    let first = lines.first()?;
    if line_text(text, first) != "---" {
        return None;
    }
    for (index, line) in lines.iter().enumerate().skip(1) {
        if line_text(text, line) == "---" {
            return Some(index + 1);
        }
    }
    None
}

fn scan_blocks(
    text: &str,
    lines: &[Line],
    mut index: usize,
    end: usize,
    parent_path: &mut Vec<usize>,
    close: Option<(usize, &str)>,
    nodes: &mut Vec<CstNode>,
    diagnostics: &mut Vec<CstDiagnostic>,
) -> usize {
    let mut sibling = 0;
    while index < end {
        let line = line_text(text, &lines[index]);
        if let Some((colons, name)) = close {
            if let Some(label) = parse_close(line, colons) {
                if let Some(label) = label {
                    if label != name {
                        diagnostics.push(CstDiagnostic {
                            message: format!("Closing delimiter does not match `{name}`."),
                            byte_offset: lines[index].start,
                        });
                    }
                }
                return index + 1;
            }
            if is_mismatched_close(line, colons) {
                diagnostics.push(CstDiagnostic {
                    message: format!("Closing delimiter does not match `{name}`."),
                    byte_offset: lines[index].start,
                });
                return index + 1;
            }
        } else if is_any_close(line) {
            diagnostics.push(CstDiagnostic {
                message: "Unmatched block closer.".to_string(),
                byte_offset: lines[index].start,
            });
            index += 1;
            continue;
        }
        if line.trim().is_empty() {
            index += 1;
            continue;
        }
        parent_path.push(sibling);
        let path = parent_path.clone();
        parent_path.pop();
        sibling += 1;
        if let Some(open) = parse_opener(line, &lines[index]) {
            let start = lines[index].start;
            if matches!(open.name.as_str(), "code" | "pre" | "math" | "style") {
                let mut cursor = index + 1;
                while cursor < end
                    && parse_close(line_text(text, &lines[cursor]), open.colons).is_none()
                {
                    cursor += 1;
                }
                let node_end = if cursor < end {
                    let close_end = lines[cursor].end;
                    cursor += 1;
                    close_end
                } else {
                    diagnostics.push(CstDiagnostic {
                        message: "Unclosed literal block.".to_string(),
                        byte_offset: start,
                    });
                    text.len()
                };
                nodes.push(CstNode {
                    kind: open.name,
                    ast_path: path,
                    byte_range: start..node_end,
                    opener_range: Some(lines[index].start..lines[index].content_end),
                    attr_range: open.attr_range,
                    attr_insert: Some(lines[index].content_end),
                });
                index = cursor;
            } else {
                let node_index = nodes.len();
                nodes.push(CstNode {
                    kind: open.name.clone(),
                    ast_path: path.clone(),
                    byte_range: start..text.len(),
                    opener_range: Some(lines[index].start..lines[index].content_end),
                    attr_range: open.attr_range,
                    attr_insert: Some(lines[index].content_end),
                });
                let mut child_path = path;
                let next = scan_blocks(
                    text,
                    lines,
                    index + 1,
                    end,
                    &mut child_path,
                    Some((open.colons, open.name.as_str())),
                    nodes,
                    diagnostics,
                );
                let node_end = if next > index + 1 {
                    lines[next - 1].end
                } else {
                    diagnostics.push(CstDiagnostic {
                        message: "Unclosed delimited block.".to_string(),
                        byte_offset: start,
                    });
                    text.len()
                };
                nodes[node_index].byte_range.end = node_end;
                index = next;
            }
            continue;
        }
        if let Some(heading) = parse_heading(line, &lines[index]) {
            nodes.push(CstNode {
                kind: "heading".to_string(),
                ast_path: path,
                byte_range: lines[index].start..lines[index].end,
                opener_range: None,
                attr_range: heading.attr_range,
                attr_insert: Some(lines[index].content_end),
            });
            index += 1;
            continue;
        }
        if is_list_start(line) {
            let start = lines[index].start;
            let kind = list_kind(line);
            index += 1;
            while index < end && list_kind(line_text(text, &lines[index])) == kind {
                index += 1;
            }
            nodes.push(CstNode {
                kind: "list".to_string(),
                ast_path: path,
                byte_range: start..lines[index - 1].end,
                opener_range: None,
                attr_range: None,
                attr_insert: None,
            });
            continue;
        }
        if index + 1 < end && is_pipe_table_header(line, line_text(text, &lines[index + 1])) {
            let start = lines[index].start;
            index += 2;
            while index < end && line_text(text, &lines[index]).trim_start().starts_with('|') {
                index += 1;
            }
            nodes.push(CstNode {
                kind: "table".to_string(),
                ast_path: path,
                byte_range: start..lines[index - 1].end,
                opener_range: None,
                attr_range: None,
                attr_insert: None,
            });
            continue;
        }
        let start = lines[index].start;
        index += 1;
        while index < end {
            let next = line_text(text, &lines[index]);
            if next.trim().is_empty()
                || parse_opener(next, &lines[index]).is_some()
                || parse_heading(next, &lines[index]).is_some()
                || is_list_start(next)
                || is_any_close(next)
                || (index + 1 < end
                    && is_pipe_table_header(next, line_text(text, &lines[index + 1])))
            {
                break;
            }
            index += 1;
        }
        nodes.push(CstNode {
            kind: "paragraph".to_string(),
            ast_path: path,
            byte_range: start..lines[index - 1].end,
            opener_range: None,
            attr_range: None,
            attr_insert: None,
        });
    }
    if let Some((_colons, name)) = close {
        diagnostics.push(CstDiagnostic {
            message: format!("Unclosed delimited block `{name}`."),
            byte_offset: lines
                .get(index.saturating_sub(1))
                .map_or(0, |line| line.end),
        });
    }
    index
}

struct Opener {
    colons: usize,
    name: String,
    attr_range: Option<Range<usize>>,
}

struct Heading {
    attr_range: Option<Range<usize>>,
}

fn line_text<'a>(text: &'a str, line: &Line) -> &'a str {
    &text[line.start..line.content_end]
}

fn parse_opener(line: &str, span: &Line) -> Option<Opener> {
    let colons = line.chars().take_while(|ch| *ch == ':').count();
    if colons < 3 {
        return None;
    }
    let rest = &line[colons..];
    let mut parts = rest.splitn(2, ' ');
    let name = parts.next()?.trim();
    if name.is_empty() || !valid_name(name, true) {
        return None;
    }
    let attr_range = parts
        .next()
        .and_then(|raw| attr_range(raw, span.content_end - raw.len()));
    Some(Opener {
        colons,
        name: name.to_string(),
        attr_range,
    })
}

fn parse_heading(line: &str, span: &Line) -> Option<Heading> {
    let level = line.chars().take_while(|ch| *ch == '#').count();
    if !(1..=6).contains(&level) || !line[level..].starts_with(' ') {
        return None;
    }
    let raw_start = level + 1;
    let raw = line[raw_start..].trim_end();
    let attr_range = raw.rfind(" {").and_then(|pos| {
        if raw.ends_with('}') {
            let absolute = span.start + raw_start + pos + 1;
            Some(absolute..span.start + raw_start + raw.len())
        } else {
            None
        }
    });
    Some(Heading { attr_range })
}

fn attr_range(raw: &str, raw_start: usize) -> Option<Range<usize>> {
    let start_trim = raw.len() - raw.trim_start().len();
    let end_trim = raw.trim_end().len();
    let trimmed = &raw[start_trim..end_trim];
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        Some(raw_start + start_trim..raw_start + end_trim)
    } else {
        None
    }
}

fn parse_close(line: &str, n: usize) -> Option<Option<&str>> {
    let colons = line.chars().take_while(|ch| *ch == ':').count();
    if colons != n {
        return None;
    }
    let after = &line[n..];
    if after.trim().is_empty() {
        return Some(None);
    }
    after
        .strip_prefix(' ')
        .map(str::trim_end)
        .filter(|label| valid_name(label, true))
        .map(Some)
}

fn is_mismatched_close(line: &str, n: usize) -> bool {
    line.chars().take_while(|ch| *ch == ':').count() == n && parse_close(line, n).is_none()
}

fn is_any_close(line: &str) -> bool {
    let colons = line.chars().take_while(|ch| *ch == ':').count();
    colons >= 3 && line[colons..].trim().is_empty()
}

fn is_list_start(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("- ")
        || t.starts_with("* ")
        || t.starts_with("+ ")
        || t.starts_with("- [ ] ")
        || t.starts_with("- [x] ")
        || ordered_marker(t).is_some()
}

fn list_kind(line: &str) -> &'static str {
    if ordered_marker(line.trim_start()).is_some() {
        "ordered"
    } else if line.trim_start().starts_with("- [") {
        "task"
    } else {
        "bullet"
    }
}

fn ordered_marker(line: &str) -> Option<usize> {
    let (digits, rest) = line.split_once('.')?;
    if !digits.is_empty() && digits.chars().all(|ch| ch.is_ascii_digit()) && rest.starts_with(' ') {
        Some(digits.len() + 2)
    } else {
        None
    }
}

fn is_pipe_table_header(line: &str, next: &str) -> bool {
    line.trim_start().starts_with('|') && next.chars().all(|ch| matches!(ch, '|' | '-' | ':' | ' '))
}

fn quote_attr_value(value: &str) -> String {
    let mut out = String::from("\"");
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}

fn find_attr_value(attr_text: &str, base: usize, name: &str) -> Option<Range<usize>> {
    find_attr(attr_text, base, name).map(|attr| attr.value)
}

fn find_attr_token(attr_text: &str, base: usize, name: &str) -> Option<Range<usize>> {
    find_attr(attr_text, base, name).map(|attr| attr.token)
}

struct AttrMatch {
    token: Range<usize>,
    value: Range<usize>,
}

fn find_attr(attr_text: &str, base: usize, name: &str) -> Option<AttrMatch> {
    let bytes = attr_text.as_bytes();
    let mut i = 1;
    while i + 1 < bytes.len() {
        while i + 1 < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let token_start = i;
        let mut quoted = false;
        while i + 1 < bytes.len() {
            match bytes[i] {
                b'"' => quoted = !quoted,
                b' ' | b'\t' if !quoted => break,
                _ => {}
            }
            i += 1;
        }
        let token_end = i;
        let token = &attr_text[token_start..token_end];
        if let Some(eq) = token.find('=') {
            if &token[..eq] == name {
                return Some(AttrMatch {
                    token: base + token_start..base + token_end,
                    value: base + token_start + eq + 1..base + token_end,
                });
            }
        }
        i += 1;
    }
    None
}

fn ast_node<'a>(nodes: &'a [Node], path: &[usize]) -> Option<&'a Node> {
    let (first, rest) = path.split_first()?;
    let mut node = nodes.get(*first)?;
    for index in rest {
        node = node.children.get(*index)?;
    }
    Some(node)
}

fn resolve_target_path(doc: &Document, target: &Target) -> Result<Vec<usize>, CstError> {
    let mut matches = Vec::new();
    collect_target_matches(&doc.body, target, &mut Vec::new(), &mut matches);
    match matches.len() {
        1 => Ok(matches.remove(0)),
        0 => Err(CstError::MissingNode),
        _ => Err(CstError::UnsupportedPatch),
    }
}

fn collect_target_matches(
    nodes: &[Node],
    target: &Target,
    path: &mut Vec<usize>,
    matches: &mut Vec<Vec<usize>>,
) {
    for (index, node) in nodes.iter().enumerate() {
        path.push(index);
        let found = match target {
            Target::Id(id) => node.id.as_deref() == Some(id.as_str()),
            Target::Path(target_path) => path == target_path,
            Target::Hash(hash) => nodx_agent_sdk::node_hash(node) == *hash,
        };
        if found {
            matches.push(path.clone());
        }
        collect_target_matches(&node.children, target, path, matches);
        path.pop();
    }
}

#[cfg(test)]
mod tests {
    use nodx_agent_sdk::{Batch, Operation, Target, apply_batch, node_hash};

    use super::*;

    #[test]
    fn emits_original_bytes_for_mixed_fixture() {
        let source = include_bytes!("../tests/fixtures/roundtrip_mixed.nodx");
        let cst = parse_bytes(source).unwrap();

        assert_eq!(cst.emit(), source);
        assert!(
            cst.diagnostics
                .iter()
                .any(|diagnostic| { diagnostic.message == "Unclosed delimited block `broken`." })
        );
        assert_eq!(
            cst.ast_node(&[1]).map(|node| node.node_type.as_str()),
            Some("paragraph")
        );
        assert_eq!(
            cst.cst_node(&[2, 0]).map(|node| node.kind.as_str()),
            Some("paragraph")
        );
    }

    #[test]
    fn emits_original_crlf_bytes() {
        let source = b"---\r\nschema: nodx/1.0\r\n---\r\n# CRLF {#title}\r\n\r\nParagraph.\r\n";
        let cst = parse_bytes(source).unwrap();

        assert_eq!(cst.emit(), source);
    }

    #[test]
    fn set_attribute_replaces_only_value_bytes() {
        let source = include_bytes!("../tests/fixtures/roundtrip_mixed.nodx");
        let cst = parse_bytes(source).unwrap();
        let patch = cst.set_named_attribute(&[2], "owner", "editor").unwrap();

        assert_eq!(patch.patches().len(), 1);
        let patched = cst.apply_patches(&patch).unwrap();
        let out = std::str::from_utf8(patched.emit()).unwrap();

        assert!(out.contains(":::note {#note tone=\"info\" owner=\"editor\"}"));
        assert!(out.contains("Paragraph with **inline** text."));
    }

    #[test]
    fn set_attribute_appends_inside_existing_attr_block() {
        let source = b"# Title {#title .b .a dataz=\"last\"}\n";
        let cst = parse_bytes(source).unwrap();
        let patched = cst
            .apply_patches(&cst.set_named_attribute(&[0], "dataa", "first").unwrap())
            .unwrap();

        assert_eq!(
            std::str::from_utf8(patched.emit()).unwrap(),
            "# Title {#title .b .a dataz=\"last\" dataa=\"first\"}\n"
        );
    }

    #[test]
    fn insert_and_delete_patch_ranges_are_local() {
        let cst = parse_str_lossless("# Title\n\nParagraph.\n");
        let inserted = cst
            .apply_patches(&PatchSet::insert(8, b" {#title}".to_vec()))
            .unwrap();
        let deleted = inserted.apply_patches(&PatchSet::delete(8..17)).unwrap();

        assert_eq!(deleted.emit(), b"# Title\n\nParagraph.\n");
    }

    #[test]
    fn applies_supported_agent_attribute_batch_as_minimal_rewrite() {
        let source = b"---\nschema: nodx/1.0\n---\n# Title {#title}\n\n:::note {#note tone=\"info\"}\nReview me.\n:::\n";
        let cst = parse_bytes(source).unwrap();
        let before_hash = node_hash(cst.ast_node(&[1]).unwrap());
        let batch = Batch::new(vec![Operation::SetAttribute {
            target: Target::id("note"),
            before_hash,
            name: "tone".to_string(),
            value: "warning".to_string(),
        }]);

        let (patched, report) = cst.apply_agent_batch_minimal(&batch).unwrap();

        assert_eq!(report.records.len(), 1);
        assert_eq!(
            std::str::from_utf8(patched.emit()).unwrap(),
            "---\nschema: nodx/1.0\n---\n# Title {#title}\n\n:::note {#note tone=\"warning\"}\nReview me.\n:::\n"
        );
    }

    #[test]
    fn applies_agent_attribute_batch_sequentially_after_offsets_shift() {
        let source =
            b"# Title {#title}\n\n:::note {#note tone=\"info\" owner=\"docs\"}\nReview me.\n:::\n";
        let cst = parse_bytes(source).unwrap();
        let note_hash = node_hash(cst.ast_node(&[1]).unwrap());
        let mut ast_after_first = cst.ast().clone();
        apply_batch(
            &mut ast_after_first,
            &Batch::new(vec![Operation::SetAttribute {
                target: Target::id("note"),
                before_hash: note_hash.clone(),
                name: "tone".to_string(),
                value: "critical-warning".to_string(),
            }]),
        )
        .unwrap();
        let after_first_hash = node_hash(ast_node(&ast_after_first.body, &[1]).unwrap());
        let batch = Batch::new(vec![
            Operation::SetAttribute {
                target: Target::id("note"),
                before_hash: note_hash,
                name: "tone".to_string(),
                value: "critical-warning".to_string(),
            },
            Operation::RemoveAttribute {
                target: Target::id("note"),
                before_hash: after_first_hash,
                name: "owner".to_string(),
            },
        ]);

        let (patched, report) = cst.apply_agent_batch_minimal(&batch).unwrap();

        assert_eq!(report.records.len(), 2);
        assert_eq!(
            std::str::from_utf8(patched.emit()).unwrap(),
            "# Title {#title}\n\n:::note {#note tone=\"critical-warning\"}\nReview me.\n:::\n"
        );
    }

    #[test]
    fn delete_operation_rewrites_minimal_source_range() {
        let cst = parse_str_lossless("# Keep {#keep}\n\n# Drop {#drop}\n");
        let drop_hash = node_hash(cst.ast_node(&[1]).unwrap());
        let batch = Batch::new(vec![Operation::Delete {
            target: Target::id("drop"),
            before_hash: drop_hash,
        }]);
        let (patched, _report) = cst.apply_agent_batch_minimal(&batch).expect("delete");
        assert_eq!(
            std::str::from_utf8(patched.emit()).unwrap(),
            "# Keep {#keep}\n\n"
        );
    }

    #[test]
    fn approve_operation_writes_status_through_cst() {
        let cst = parse_str_lossless("# Title {#title fallback=\"children\"}\n");
        let hash = node_hash(cst.ast_node(&[0]).unwrap());
        let batch = Batch::new(vec![Operation::Approve {
            target: Target::id("title"),
            before_hash: hash,
            reviewer: Some("reviewer-1".to_string()),
        }]);
        let (patched, _) = cst.apply_agent_batch_minimal(&batch).expect("approve");
        let text = std::str::from_utf8(patched.emit()).unwrap();
        assert!(text.contains("status=\"approved\""));
        assert!(text.contains("reviewed-by=\"reviewer-1\""));
    }

    #[test]
    fn replace_operation_emits_minimal_paragraph() {
        let cst = parse_str_lossless(
            "---\nschema: nodx/1.0\n---\n\n# Title {#title}\n\nKeep this.\n",
        );
        let hash = node_hash(cst.ast_node(&[0]).unwrap());
        let batch = Batch::new(vec![Operation::Replace {
            target: Target::id("title"),
            before_hash: hash,
            node: nodx_core::Node {
                node_type: "paragraph".to_string(),
                id: None,
                classes: Vec::new(),
                attrs: std::collections::BTreeMap::new(),
                children: Vec::new(),
                inlines: vec![nodx_core::Inline::Text("Replacement.".to_string())],
                text: None,
            },
        }]);
        let result = cst.apply_agent_batch_minimal(&batch);
        if let Err(err) = &result {
            panic!("replace failed: {:?}", err);
        }
        let (patched, _) = result.unwrap();
        let text = std::str::from_utf8(patched.emit()).unwrap();
        eprintln!("--- patched ---\n{text}\n---");
        assert!(text.contains("Replacement."));
        assert!(!text.contains("# Title"));
    }

    #[test]
    fn append_child_insertion_is_explicitly_unsupported() {
        let cst = parse_str_lossless("# Title {#title}\n");
        let hash = node_hash(cst.ast_node(&[0]).unwrap());
        let batch = Batch::new(vec![Operation::Insert {
            target: Target::id("title"),
            position: nodx_agent_sdk::InsertPosition::AppendChild,
            before_hash: hash,
            node: nodx_core::Node {
                node_type: "paragraph".to_string(),
                id: None,
                classes: Vec::new(),
                attrs: std::collections::BTreeMap::new(),
                children: Vec::new(),
                inlines: vec![nodx_core::Inline::Text("child".to_string())],
                text: None,
            },
        }]);
        assert_eq!(
            cst.apply_agent_batch_minimal(&batch).unwrap_err(),
            CstError::UnsupportedPatch
        );
    }
}

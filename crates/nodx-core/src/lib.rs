#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub code: String,
    pub severity: String,
    pub message: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub target: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Document {
    pub schema: String,
    pub meta: BTreeMap<String, Value>,
    pub body: Vec<Node>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Node {
    pub node_type: String,
    pub id: Option<String>,
    pub classes: Vec<String>,
    pub attrs: BTreeMap<String, String>,
    pub children: Vec<Node>,
    pub inlines: Vec<Inline>,
    pub text: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Inline {
    Text(String),
    Strong(Vec<Inline>),
    Em(Vec<Inline>),
    Code(String),
    Link { label: Vec<Inline>, target: String },
    Span { children: Vec<Inline>, attrs: Attrs },
    Var { namespace: String, name: String },
    Ref { target: String },
    Mention { kind: String, target: String },
    FootnoteRef { target: String },
    CitationRef { target: String },
    MathInline { source: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Attrs {
    pub id: Option<String>,
    pub classes: Vec<String>,
    pub attrs: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(String),
    String(String),
    List(Vec<Value>),
    Map(BTreeMap<String, Value>),
}

pub fn parse_str(input: &str) -> Document {
    let mut diagnostics = Vec::new();
    if input.starts_with('\u{feff}') {
        diagnostics.push(diag(
            "NODX-E018",
            "fatal",
            "Byte Order Mark is not allowed.",
            1,
            1,
        ));
    }
    if input.contains('\0') {
        diagnostics.push(diag("NODX-E002", "fatal", "U+0000 is not allowed.", 1, 1));
    }

    let normalized = input.replace("\r\n", "\n");
    let mut lines: Vec<&str> = normalized.split('\n').collect();
    if lines.last() == Some(&"") {
        lines.pop();
    }

    let mut meta = BTreeMap::new();
    let mut start = 0;
    if lines.first() == Some(&"---") {
        match lines.iter().skip(1).position(|line| *line == "---") {
            Some(end_rel) => {
                let end = end_rel + 1;
                meta = parse_front_matter(&lines[1..end], &mut diagnostics);
                start = end + 1;
            }
            None => diagnostics.push(diag("NODX-E003", "fatal", "Unclosed front matter.", 1, 1)),
        }
    }
    meta.entry("schema".to_string())
        .or_insert(Value::String("nodx/0.1".to_string()));
    meta.entry("type".to_string())
        .or_insert(Value::String("document".to_string()));
    meta.entry("dir".to_string())
        .or_insert(Value::String("auto".to_string()));
    meta.entry("language".to_string())
        .or_insert(Value::String("und".to_string()));

    let mut parser = Parser {
        lines: &lines,
        pos: start,
        diagnostics,
    };
    let body = parser.parse_until(None);
    if !meta.contains_key("title") {
        if let Some(title) = first_heading_text(&body) {
            meta.insert("title".to_string(), Value::String(title));
        } else {
            meta.insert("title".to_string(), Value::String("Untitled".to_string()));
        }
    }

    let mut diagnostics = parser.diagnostics;
    audit_nods(&body, &mut diagnostics);

    Document {
        schema: "nodx/0.1".to_string(),
        meta,
        body,
        diagnostics,
    }
}

pub fn parse_bytes(input: &[u8]) -> Result<Document, Diagnostic> {
    if input.starts_with(b"PK\x03\x04") {
        let entry = read_packaged_nodx_entry(input)?;
        return parse_bytes(&entry);
    }
    match std::str::from_utf8(input) {
        Ok(s) => Ok(parse_str(s)),
        Err(_) => Err(Diagnostic {
            code: "NODX-E001".to_string(),
            severity: "fatal".to_string(),
            message: "Input is not valid UTF-8.".to_string(),
            line: Some(1),
            column: Some(1),
            target: None,
        }),
    }
}

pub fn is_packaged_nodx(input: &[u8]) -> bool {
    input.starts_with(b"PK\x03\x04")
}

fn read_packaged_nodx_entry(input: &[u8]) -> Result<Vec<u8>, Diagnostic> {
    let entries = zip_entries(input)?;
    let first = entries
        .iter()
        .min_by_key(|entry| entry.local_offset)
        .ok_or_else(|| package_diag("Package is empty."))?;
    if first.name != "mimetype" {
        return Err(package_diag("First ZIP entry must be mimetype."));
    }
    let mimetype = zip_read_stored(input, first)?;
    if mimetype != b"application/nodx+zip" {
        return Err(package_diag("Invalid NODX package mimetype."));
    }

    let manifest = entries
        .iter()
        .find(|entry| entry.name == "manifest.yaml")
        .ok_or_else(|| package_diag("Package is missing manifest.yaml."))?;
    let manifest_text = String::from_utf8(zip_read_stored(input, manifest)?)
        .map_err(|_| package_diag("Package manifest is not UTF-8."))?;
    let entry_path = manifest_entry_path(&manifest_text)
        .ok_or_else(|| package_diag("Package manifest is missing entry."))?;
    let doc_entry = entries
        .iter()
        .find(|entry| entry.name == entry_path)
        .ok_or_else(|| package_diag("Package entry document is missing."))?;
    zip_read_stored(input, doc_entry)
}

struct ZipEntry {
    name: String,
    compression: u16,
    compressed_size: usize,
    uncompressed_size: usize,
    local_offset: usize,
}

fn zip_entries(input: &[u8]) -> Result<Vec<ZipEntry>, Diagnostic> {
    let eocd =
        find_eocd(input).ok_or_else(|| package_diag("ZIP end of central directory not found."))?;
    let count = read_u16(input, eocd + 10)? as usize;
    let cd_offset = read_u32(input, eocd + 16)? as usize;
    let mut pos = cd_offset;
    let mut entries = Vec::new();
    let mut seen_names = BTreeSet::new();
    for _ in 0..count {
        if read_u32(input, pos)? != 0x0201_4b50 {
            return Err(package_diag("Invalid ZIP central directory."));
        }
        let compression = read_u16(input, pos + 10)?;
        let compressed_size = read_u32(input, pos + 20)? as usize;
        let uncompressed_size = read_u32(input, pos + 24)? as usize;
        let name_len = read_u16(input, pos + 28)? as usize;
        let extra_len = read_u16(input, pos + 30)? as usize;
        let comment_len = read_u16(input, pos + 32)? as usize;
        let local_offset = read_u32(input, pos + 42)? as usize;
        let name_start = pos + 46;
        let name_end = checked_add(name_start, name_len)?;
        if name_end > input.len() {
            return Err(package_diag("ZIP entry name is out of bounds."));
        }
        let name = std::str::from_utf8(&input[name_start..name_end])
            .map_err(|_| package_diag("ZIP entry name is not UTF-8."))?
            .to_string();
        validate_package_path(&name)?;
        if !seen_names.insert(name.clone()) {
            return Err(package_diag("Duplicate package entry path."));
        }
        entries.push(ZipEntry {
            name,
            compression,
            compressed_size,
            uncompressed_size,
            local_offset,
        });
        pos = checked_add(name_end, checked_add(extra_len, comment_len)?)?;
    }
    Ok(entries)
}

fn zip_read_stored(input: &[u8], entry: &ZipEntry) -> Result<Vec<u8>, Diagnostic> {
    if entry.compression != 0 {
        return Err(package_diag(
            "This minimal reference reader supports only stored ZIP entries.",
        ));
    }
    if entry.compressed_size != entry.uncompressed_size {
        return Err(package_diag("Stored ZIP entry has inconsistent sizes."));
    }
    let pos = entry.local_offset;
    if read_u32(input, pos)? != 0x0403_4b50 {
        return Err(package_diag("Invalid ZIP local header."));
    }
    let name_len = read_u16(input, pos + 26)? as usize;
    let extra_len = read_u16(input, pos + 28)? as usize;
    let data_start = checked_add(pos + 30, checked_add(name_len, extra_len)?)?;
    let data_end = checked_add(data_start, entry.uncompressed_size)?;
    if data_end > input.len() {
        return Err(package_diag("ZIP entry data is out of bounds."));
    }
    Ok(input[data_start..data_end].to_vec())
}

fn find_eocd(input: &[u8]) -> Option<usize> {
    let min = 22;
    if input.len() < min {
        return None;
    }
    let start = input.len().saturating_sub(65_557);
    (start..=input.len() - min)
        .rev()
        .find(|&pos| input.get(pos..pos + 4) == Some(b"PK\x05\x06"))
}

fn manifest_entry_path(manifest: &str) -> Option<String> {
    manifest.lines().find_map(|line| {
        let trimmed = line.trim();
        trimmed
            .strip_prefix("entry:")
            .map(|value| value.trim().trim_matches('"').to_string())
    })
}

fn validate_package_path(path: &str) -> Result<(), Diagnostic> {
    if path.is_empty()
        || path.starts_with('/')
        || path.contains('\\')
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        Err(package_diag("Unsafe package path."))
    } else {
        Ok(())
    }
}

fn read_u16(input: &[u8], pos: usize) -> Result<u16, Diagnostic> {
    let bytes = input
        .get(pos..pos + 2)
        .ok_or_else(|| package_diag("Unexpected end of ZIP data."))?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn read_u32(input: &[u8], pos: usize) -> Result<u32, Diagnostic> {
    let bytes = input
        .get(pos..pos + 4)
        .ok_or_else(|| package_diag("Unexpected end of ZIP data."))?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn checked_add(a: usize, b: usize) -> Result<usize, Diagnostic> {
    a.checked_add(b)
        .ok_or_else(|| package_diag("ZIP offset overflow."))
}

fn package_diag(message: &str) -> Diagnostic {
    Diagnostic {
        code: "NODX-E022".to_string(),
        severity: "fatal".to_string(),
        message: message.to_string(),
        line: None,
        column: None,
        target: None,
    }
}

struct Parser<'a> {
    lines: &'a [&'a str],
    pos: usize,
    diagnostics: Vec<Diagnostic>,
}

impl Parser<'_> {
    fn parse_until(&mut self, close_frame: Option<(usize, &str)>) -> Vec<Node> {
        let mut out = Vec::new();
        let mut closed = close_frame.is_none();
        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            if let Some((n, expected_name)) = close_frame {
                if let Some(label) = parse_close(line, n) {
                    if let Some(name) = label {
                        if name != expected_name {
                            self.diagnostics.push(diag(
                                "NODX-E005",
                                "error",
                                &format!(
                                    "Closing label `{}` does not match open block `{}`.",
                                    name, expected_name
                                ),
                                self.pos + 1,
                                1,
                            ));
                        }
                    }
                    self.pos += 1;
                    closed = true;
                    break;
                }
            } else if is_any_close(line) {
                self.diagnostics.push(diag(
                    "NODX-E005",
                    "error",
                    "Unmatched block closer.",
                    self.pos + 1,
                    1,
                ));
                self.pos += 1;
                continue;
            }

            if line.trim().is_empty() {
                self.pos += 1;
                continue;
            }
            if let Some((colons, name, attrs)) = parse_opener(line) {
                out.push(self.parse_delimited(colons, name, attrs));
                continue;
            }
            if let Some((level, content, attrs)) = parse_heading(line) {
                self.pos += 1;
                let mut attr_map = attrs.unwrap_or_default();
                attr_map
                    .attrs
                    .insert("level".to_string(), level.to_string());
                out.push(Node::textual("heading", attr_map, parse_inlines(content)));
                continue;
            }
            if is_list_start(line) {
                out.push(self.parse_list());
                continue;
            }
            if self.pos + 1 < self.lines.len()
                && is_pipe_table_header(line, self.lines[self.pos + 1])
            {
                out.push(self.parse_pipe_table());
                continue;
            }
            out.push(self.parse_paragraph());
        }
        if !closed {
            self.diagnostics.push(diag(
                "NODX-E005",
                "error",
                "Unclosed delimited block at end of input.",
                self.pos.max(1),
                1,
            ));
        }
        out
    }

    fn parse_delimited(&mut self, colons: usize, name: String, attrs: Attrs) -> Node {
        self.pos += 1;
        if matches!(name.as_str(), "code" | "pre" | "math" | "style") {
            let start = self.pos;
            while self.pos < self.lines.len()
                && parse_close(self.lines[self.pos], colons).is_none()
            {
                self.pos += 1;
            }
            let text = self.lines[start..self.pos].join("\n");
            if self.pos < self.lines.len() {
                if let Some(Some(label)) = parse_close(self.lines[self.pos], colons) {
                    if label != name {
                        self.diagnostics.push(diag(
                            "NODX-E005",
                            "error",
                            &format!(
                                "Closing label `{}` does not match open block `{}`.",
                                label, name
                            ),
                            self.pos + 1,
                            1,
                        ));
                    }
                }
                self.pos += 1;
            } else {
                self.diagnostics.push(diag(
                    "NODX-E005",
                    "error",
                    "Unclosed literal block.",
                    start + 1,
                    1,
                ));
            }
            return Node::literal(&name, attrs, text);
        }

        let children = self.parse_until(Some((colons, name.as_str())));
        Node::container(&name, attrs, children)
    }

    fn parse_list(&mut self) -> Node {
        let first = self.lines[self.pos];
        let kind = list_kind(first);
        let mut items = Vec::new();
        while self.pos < self.lines.len() && list_kind(self.lines[self.pos]) == kind {
            let raw = self.lines[self.pos];
            let (content, checked) = strip_list_marker(raw);
            self.pos += 1;
            let mut parts = vec![content.to_string()];
            while self.pos < self.lines.len() && self.lines[self.pos].starts_with("  ") {
                parts.push(self.lines[self.pos].trim_start().to_string());
                self.pos += 1;
            }
            let mut attrs = Attrs::default();
            if let Some(done) = checked {
                attrs.attrs.insert("checked".to_string(), done.to_string());
            }
            items.push(Node::textual(
                "item",
                attrs,
                parse_inlines(&parts.join("\n")),
            ));
        }
        let mut attrs = Attrs::default();
        attrs
            .attrs
            .insert("kind".to_string(), kind.unwrap_or("unordered").to_string());
        Node::container("list", attrs, items)
    }

    fn parse_pipe_table(&mut self) -> Node {
        let header = split_pipe_row(self.lines[self.pos]);
        self.pos += 2;
        let mut rows = vec![table_row(header, true)];
        while self.pos < self.lines.len()
            && self.lines[self.pos].contains('|')
            && !self.lines[self.pos].trim().is_empty()
        {
            rows.push(table_row(split_pipe_row(self.lines[self.pos]), false));
            self.pos += 1;
        }
        Node::container("table", Attrs::default(), rows)
    }

    fn parse_paragraph(&mut self) -> Node {
        let start = self.pos;
        self.pos += 1;
        while self.pos < self.lines.len()
            && !self.lines[self.pos].trim().is_empty()
            && parse_opener(self.lines[self.pos]).is_none()
            && parse_heading(self.lines[self.pos]).is_none()
            && !is_list_start(self.lines[self.pos])
            && !is_any_close(self.lines[self.pos])
        {
            if self.pos + 1 < self.lines.len()
                && is_pipe_table_header(self.lines[self.pos], self.lines[self.pos + 1])
            {
                break;
            }
            self.pos += 1;
        }
        Node::textual(
            "paragraph",
            Attrs::default(),
            parse_inlines(&self.lines[start..self.pos].join("\n")),
        )
    }
}

impl Node {
    fn textual(node_type: &str, attrs: Attrs, inlines: Vec<Inline>) -> Self {
        Self::base(node_type, attrs, Vec::new(), inlines, None)
    }

    fn literal(node_type: &str, attrs: Attrs, text: String) -> Self {
        Self::base(node_type, attrs, Vec::new(), Vec::new(), Some(text))
    }

    fn container(node_type: &str, attrs: Attrs, children: Vec<Node>) -> Self {
        Self::base(node_type, attrs, children, Vec::new(), None)
    }

    fn base(
        node_type: &str,
        attrs: Attrs,
        children: Vec<Node>,
        inlines: Vec<Inline>,
        text: Option<String>,
    ) -> Self {
        Self {
            node_type: node_type.to_string(),
            id: attrs.id,
            classes: attrs.classes,
            attrs: attrs.attrs,
            children,
            inlines,
            text,
        }
    }
}

impl Default for Attrs {
    fn default() -> Self {
        Self {
            id: None,
            classes: Vec::new(),
            attrs: BTreeMap::new(),
        }
    }
}

fn parse_front_matter(
    lines: &[&str],
    diagnostics: &mut Vec<Diagnostic>,
) -> BTreeMap<String, Value> {
    let mut map = BTreeMap::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if line.trim().is_empty() {
            i += 1;
            continue;
        }
        check_yaml_safety(line, i + 2, diagnostics);
        if !line.starts_with(' ') {
            if let Some((k, v)) = line.split_once(':') {
                let key = k.trim().to_string();
                if map.contains_key(&key) {
                    diagnostics.push(diag(
                        "NODX-E020",
                        "fatal",
                        "Duplicate front matter key.",
                        i + 2,
                        1,
                    ));
                }
                let rest = v.trim();
                if rest.is_empty() {
                    let next = lines.get(i + 1).copied().unwrap_or("");
                    if next.trim_start().starts_with("- ") || next.trim() == "-" {
                        let (list, consumed) = parse_block_sequence(lines, i + 1, diagnostics);
                        map.insert(key, Value::List(list));
                        i = consumed;
                        continue;
                    }
                    let (child, consumed) = parse_block_mapping(lines, i + 1, diagnostics);
                    map.insert(key, Value::Map(child));
                    i = consumed;
                    continue;
                }
                map.insert(key, scalar(rest));
            }
        }
        i += 1;
    }
    map
}

fn check_yaml_safety(line: &str, line_no: usize, diagnostics: &mut Vec<Diagnostic>) {
    let trimmed = line.trim_start();
    if trimmed.starts_with('&')
        || trimmed.starts_with("*")
        || trimmed.starts_with("!!")
        || trimmed.starts_with("---")
        || trimmed.starts_with("<<:")
    {
        diagnostics.push(diag(
            "NODX-E019",
            "fatal",
            "Forbidden YAML safe-subset construct.",
            line_no,
            1,
        ));
    }
}

fn parse_block_mapping(
    lines: &[&str],
    start: usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> (BTreeMap<String, Value>, usize) {
    let mut map = BTreeMap::new();
    let mut i = start;
    while i < lines.len() && lines[i].starts_with("  ") && !lines[i].trim_start().starts_with("- ") {
        check_yaml_safety(lines[i], i + 2, diagnostics);
        if let Some((ck, cv)) = lines[i].trim().split_once(':') {
            map.insert(ck.trim().to_string(), scalar(cv.trim()));
        }
        i += 1;
    }
    (map, i)
}

fn parse_block_sequence(
    lines: &[&str],
    start: usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> (Vec<Value>, usize) {
    let mut out = Vec::new();
    let mut i = start;
    while i < lines.len() {
        let line = lines[i];
        if !line.starts_with("  ") {
            break;
        }
        let trimmed = line.trim_start();
        if !trimmed.starts_with("- ") && trimmed != "-" {
            break;
        }
        check_yaml_safety(line, i + 2, diagnostics);
        let after = if trimmed == "-" { "" } else { &trimmed[2..] };
        if let Some((k, v)) = after.split_once(':') {
            let mut child = BTreeMap::new();
            let key = k.trim().to_string();
            let rest = v.trim();
            if rest.is_empty() {
                i += 1;
                while i < lines.len() && lines[i].starts_with("    ") {
                    check_yaml_safety(lines[i], i + 2, diagnostics);
                    if let Some((ck, cv)) = lines[i].trim().split_once(':') {
                        child.insert(ck.trim().to_string(), scalar(cv.trim()));
                    }
                    i += 1;
                }
                out.push(Value::Map(child));
                continue;
            }
            child.insert(key, scalar(rest));
            i += 1;
            while i < lines.len() && lines[i].starts_with("    ") && !lines[i].trim_start().starts_with("- ") {
                check_yaml_safety(lines[i], i + 2, diagnostics);
                if let Some((ck, cv)) = lines[i].trim().split_once(':') {
                    child.insert(ck.trim().to_string(), scalar(cv.trim()));
                }
                i += 1;
            }
            out.push(Value::Map(child));
        } else {
            out.push(scalar(after));
            i += 1;
        }
    }
    (out, i)
}

fn scalar(raw: &str) -> Value {
    if raw == "null" {
        Value::Null
    } else if raw == "true" {
        Value::Bool(true)
    } else if raw == "false" {
        Value::Bool(false)
    } else if raw.starts_with('[') && raw.ends_with(']') {
        let inner = &raw[1..raw.len() - 1];
        Value::List(inner.split(',').map(|s| scalar(s.trim())).collect())
    } else if raw.parse::<f64>().is_ok() && raw.chars().any(|c| c.is_ascii_digit()) {
        Value::Number(raw.to_string())
    } else {
        Value::String(unquote(raw))
    }
}

fn unquote(raw: &str) -> String {
    if (raw.starts_with('"') && raw.ends_with('"'))
        || (raw.starts_with('\'') && raw.ends_with('\''))
    {
        raw[1..raw.len() - 1].to_string()
    } else {
        raw.to_string()
    }
}

fn parse_opener(line: &str) -> Option<(usize, String, Attrs)> {
    let colons = line.chars().take_while(|c| *c == ':').count();
    if colons < 3 {
        return None;
    }
    let rest = &line[colons..];
    let mut parts = rest.splitn(2, ' ');
    let name = parts.next()?.trim();
    if name.is_empty() || !valid_name(name, true) {
        return None;
    }
    let attrs = parts.next().and_then(parse_attrs).unwrap_or_default();
    Some((colons, name.to_string(), attrs))
}

fn parse_heading(line: &str) -> Option<(usize, &str, Option<Attrs>)> {
    let level = line.chars().take_while(|c| *c == '#').count();
    if !(1..=6).contains(&level) || !line[level..].starts_with(' ') {
        return None;
    }
    let raw = line[level + 1..].trim_end();
    if let Some(pos) = raw.rfind(" {") {
        if raw.ends_with('}') {
            return Some((level, raw[..pos].trim_end(), parse_attrs(&raw[pos + 1..])));
        }
    }
    Some((level, raw, None))
}

fn parse_attrs(raw: &str) -> Option<Attrs> {
    let s = raw.trim();
    if !s.starts_with('{') || !s.ends_with('}') {
        return None;
    }
    let mut attrs = Attrs::default();
    for token in split_attr_tokens(&s[1..s.len() - 1]) {
        if let Some(id) = token.strip_prefix('#') {
            attrs.id = Some(id.to_string());
        } else if let Some(class) = token.strip_prefix('.') {
            attrs.classes.push(class.to_string());
        } else if let Some((k, v)) = token.split_once('=') {
            attrs.attrs.insert(k.to_string(), unquote(v));
        }
    }
    let mut set = BTreeSet::new();
    attrs.classes.retain(|c| set.insert(c.clone()));
    attrs.classes.sort();
    Some(attrs)
}

fn split_attr_tokens(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut quoted = false;
    for ch in input.chars() {
        match ch {
            '"' => {
                quoted = !quoted;
                buf.push(ch);
            }
            ' ' if !quoted => {
                if !buf.is_empty() {
                    out.push(std::mem::take(&mut buf));
                }
            }
            _ => buf.push(ch),
        }
    }
    if !buf.is_empty() {
        out.push(buf);
    }
    out
}

pub fn parse_inlines(input: &str) -> Vec<Inline> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < input.len() {
        let rest = &input[i..];
        if let Some(end) = rest.strip_prefix('`').and_then(|r| r.find('`')) {
            out.push(Inline::Code(rest[1..end + 1].to_string()));
            i += end + 2;
        } else if let Some(end) = rest.strip_prefix("$$").and_then(|r| r.find("$$")) {
            out.push(Inline::MathInline {
                source: rest[2..end + 2].to_string(),
            });
            i += end + 4;
        } else if let Some(end) = rest.strip_prefix("{{").and_then(|r| r.find("}}")) {
            let name = &rest[2..end + 2];
            if let Some((ns, n)) = name.split_once('.') {
                out.push(Inline::Var {
                    namespace: ns.to_string(),
                    name: n.to_string(),
                });
            } else {
                out.push(Inline::Text(rest[..end + 4].to_string()));
            }
            i += end + 4;
        } else if let Some(end) = rest.strip_prefix("[^").and_then(|r| r.find(']')) {
            out.push(Inline::FootnoteRef {
                target: rest[2..end + 2].to_string(),
            });
            i += end + 3;
        } else if let Some(end) = rest.strip_prefix("[@").and_then(|r| r.find(']')) {
            out.push(Inline::CitationRef {
                target: rest[2..end + 2].to_string(),
            });
            i += end + 3;
        } else if let Some(end) = rest.strip_prefix("@[").and_then(|r| r.find(']')) {
            out.push(Inline::Ref {
                target: rest[2..end + 2].to_string(),
            });
            i += end + 3;
        } else if let Some(end) = rest.strip_prefix("@{").and_then(|r| r.find('}')) {
            let raw = &rest[2..end + 2];
            if let Some((kind, target)) = raw.split_once(':') {
                out.push(Inline::Mention {
                    kind: kind.to_string(),
                    target: target.to_string(),
                });
            } else {
                out.push(Inline::Text(rest[..end + 3].to_string()));
            }
            i += end + 3;
        } else if rest.starts_with("**") {
            if let Some(end) = rest[2..].find("**") {
                out.push(Inline::Strong(parse_inlines(&rest[2..end + 2])));
                i += end + 4;
            } else {
                push_text(&mut out, "*");
                i += 1;
            }
        } else if let Some(stripped) = rest.strip_prefix('*') {
            if let Some(end) = stripped.find('*') {
                out.push(Inline::Em(parse_inlines(&stripped[..end])));
                i += end + 2;
            } else {
                push_text(&mut out, "*");
                i += 1;
            }
        } else if rest.starts_with('[') {
            if let Some(close) = rest.find(']') {
                let label = &rest[1..close];
                let after = &rest[close + 1..];
                if after.starts_with('(') {
                    if let Some(end) = after.find(')') {
                        out.push(Inline::Link {
                            label: parse_inlines(label),
                            target: after[1..end].to_string(),
                        });
                        i += close + 1 + end + 1;
                        continue;
                    }
                } else if after.starts_with('{') {
                    if let Some(end) = after.find('}') {
                        if let Some(attrs) = parse_attrs(&after[..=end]) {
                            out.push(Inline::Span {
                                children: parse_inlines(label),
                                attrs,
                            });
                            i += close + 1 + end + 1;
                            continue;
                        }
                    }
                }
            }
            push_text(&mut out, "[");
            i += 1;
        } else if rest.starts_with('\\') && rest.len() > 1 {
            let ch = rest[1..].chars().next().unwrap();
            if "`*[](){}#@~^=:|".contains(ch) {
                push_text(&mut out, &ch.to_string());
                i += 1 + ch.len_utf8();
            } else {
                push_text(&mut out, "\\");
                i += 1;
            }
        } else {
            let ch = rest.chars().next().unwrap();
            push_text(&mut out, &ch.to_string());
            i += ch.len_utf8();
        }
    }
    out
}

fn push_text(out: &mut Vec<Inline>, text: &str) {
    if let Some(Inline::Text(prev)) = out.last_mut() {
        prev.push_str(text);
    } else {
        out.push(Inline::Text(text.to_string()));
    }
}

fn valid_name(s: &str, allow_hyphen: bool) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || (allow_hyphen && c == '-'))
}

fn parse_close(line: &str, n: usize) -> Option<Option<String>> {
    let colons = line.chars().take_while(|c| *c == ':').count();
    if colons != n {
        return None;
    }
    let after = &line[n..];
    if after.trim().is_empty() {
        return Some(None);
    }
    if let Some(rest) = after.strip_prefix(' ') {
        let trimmed = rest.trim_end();
        if valid_name(trimmed, true) {
            return Some(Some(trimmed.to_string()));
        }
    }
    None
}

fn is_any_close(line: &str) -> bool {
    let n = line.chars().take_while(|c| *c == ':').count();
    if n < 3 {
        return false;
    }
    let after = &line[n..];
    if after.trim().is_empty() {
        return true;
    }
    if let Some(rest) = after.strip_prefix(' ') {
        return valid_name(rest.trim_end(), true);
    }
    false
}

fn is_list_start(line: &str) -> bool {
    list_kind(line).is_some()
}

fn list_kind(line: &str) -> Option<&'static str> {
    if line.starts_with("- [ ] ") || line.starts_with("- [x] ") {
        Some("task")
    } else if line.starts_with("- ") {
        Some("unordered")
    } else {
        let (n, _) = line.split_once(". ")?;
        if !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()) {
            Some("ordered")
        } else {
            None
        }
    }
}

fn strip_list_marker(line: &str) -> (&str, Option<bool>) {
    if let Some(rest) = line.strip_prefix("- [ ] ") {
        (rest, Some(false))
    } else if let Some(rest) = line.strip_prefix("- [x] ") {
        (rest, Some(true))
    } else if let Some(rest) = line.strip_prefix("- ") {
        (rest, None)
    } else {
        let (_, rest) = line.split_once(". ").unwrap();
        (rest, None)
    }
}

fn is_pipe_table_header(a: &str, b: &str) -> bool {
    a.contains('|')
        && b.trim().chars().all(|c| matches!(c, '|' | '-' | ':' | ' '))
        && b.contains('-')
}

fn split_pipe_row(line: &str) -> Vec<String> {
    line.trim()
        .trim_matches('|')
        .split('|')
        .map(|c| c.trim().to_string())
        .collect()
}

fn table_row(cells: Vec<String>, header: bool) -> Node {
    let children = cells
        .into_iter()
        .map(|cell| {
            let mut attrs = Attrs::default();
            if header {
                attrs.attrs.insert("header".to_string(), "true".to_string());
                attrs.attrs.insert("scope".to_string(), "col".to_string());
            }
            Node::textual("cell", attrs, parse_inlines(&cell))
        })
        .collect();
    Node::container("row", Attrs::default(), children)
}

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

fn audit_nods(nodes: &[Node], diagnostics: &mut Vec<Diagnostic>) {
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

fn first_heading_text(nodes: &[Node]) -> Option<String> {
    for node in nodes {
        if node.node_type == "heading" {
            return Some(plain_inlines(&node.inlines));
        }
        if let Some(found) = first_heading_text(&node.children) {
            return Some(found);
        }
    }
    None
}

fn diag(code: &str, severity: &str, message: &str, line: usize, column: usize) -> Diagnostic {
    Diagnostic {
        code: code.to_string(),
        severity: severity.to_string(),
        message: message.to_string(),
        line: Some(line),
        column: Some(column),
        target: None,
    }
}

pub fn canonical_json(doc: &Document) -> String {
    let mut s = String::new();
    write_document(&mut s, doc);
    s
}

fn write_document(out: &mut String, doc: &Document) {
    out.push_str("{\"body\":");
    write_nodes(out, &doc.body);
    out.push_str(",\"meta\":");
    write_map_value(out, &doc.meta);
    out.push_str(",\"schema\":\"");
    escape_json(out, &doc.schema);
    out.push_str("\"}");
}

fn write_nodes(out: &mut String, nodes: &[Node]) {
    out.push('[');
    for (i, node) in nodes.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        write_node(out, node);
    }
    out.push(']');
}

fn write_node(out: &mut String, node: &Node) {
    out.push_str("{\"attrs\":");
    write_str_map(out, &node.attrs);
    out.push_str(",\"children\":");
    write_nodes(out, &node.children);
    out.push_str(",\"classes\":");
    write_str_list(out, &node.classes);
    out.push_str(",\"id\":");
    match &node.id {
        Some(id) => write_json_string(out, id),
        None => out.push_str("null"),
    }
    out.push_str(",\"inlines\":");
    write_inlines(out, &node.inlines);
    out.push_str(",\"text\":");
    match &node.text {
        Some(text) => write_json_string(out, text),
        None => out.push_str("null"),
    }
    out.push_str(",\"type\":");
    write_json_string(out, &node.node_type);
    out.push('}');
}

fn write_inlines(out: &mut String, inlines: &[Inline]) {
    out.push('[');
    for (i, item) in inlines.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        match item {
            Inline::Text(text) => {
                out.push_str("{\"text\":");
                write_json_string(out, text);
                out.push_str(",\"type\":\"text\"}");
            }
            Inline::Strong(children) => inline_children(out, "strong", children),
            Inline::Em(children) => inline_children(out, "em", children),
            Inline::Code(text) => {
                out.push_str("{\"text\":");
                write_json_string(out, text);
                out.push_str(",\"type\":\"code\"}");
            }
            Inline::Link { label, target } => {
                out.push_str("{\"label\":");
                write_inlines(out, label);
                out.push_str(",\"target\":");
                write_json_string(out, target);
                out.push_str(",\"type\":\"link\"}");
            }
            Inline::Span { children, attrs } => {
                out.push_str("{\"attrs\":");
                write_attrs(out, attrs);
                out.push_str(",\"children\":");
                write_inlines(out, children);
                out.push_str(",\"type\":\"span\"}");
            }
            Inline::Var { namespace, name } => {
                out.push_str("{\"name\":");
                write_json_string(out, name);
                out.push_str(",\"namespace\":");
                write_json_string(out, namespace);
                out.push_str(",\"type\":\"var\"}");
            }
            Inline::Ref { target } => {
                out.push_str("{\"target\":");
                write_json_string(out, target);
                out.push_str(",\"type\":\"ref\"}");
            }
            Inline::Mention { kind, target } => {
                out.push_str("{\"kind\":");
                write_json_string(out, kind);
                out.push_str(",\"target\":");
                write_json_string(out, target);
                out.push_str(",\"type\":\"mention\"}");
            }
            Inline::FootnoteRef { target } => {
                out.push_str("{\"target\":");
                write_json_string(out, target);
                out.push_str(",\"type\":\"footnote-ref\"}");
            }
            Inline::CitationRef { target } => {
                out.push_str("{\"target\":");
                write_json_string(out, target);
                out.push_str(",\"type\":\"citation-ref\"}");
            }
            Inline::MathInline { source } => {
                out.push_str("{\"source\":");
                write_json_string(out, source);
                out.push_str(",\"type\":\"math-inline\"}");
            }
        }
    }
    out.push(']');
}

fn inline_children(out: &mut String, typ: &str, children: &[Inline]) {
    out.push_str("{\"children\":");
    write_inlines(out, children);
    out.push_str(",\"type\":");
    write_json_string(out, typ);
    out.push('}');
}

fn write_attrs(out: &mut String, attrs: &Attrs) {
    out.push_str("{\"attrs\":");
    write_str_map(out, &attrs.attrs);
    out.push_str(",\"classes\":");
    write_str_list(out, &attrs.classes);
    out.push_str(",\"id\":");
    match &attrs.id {
        Some(id) => write_json_string(out, id),
        None => out.push_str("null"),
    }
    out.push('}');
}

fn write_map_value(out: &mut String, map: &BTreeMap<String, Value>) {
    out.push('{');
    for (i, (k, v)) in map.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        write_json_string(out, k);
        out.push(':');
        write_value(out, v);
    }
    out.push('}');
}

fn write_value(out: &mut String, value: &Value) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(v) => out.push_str(if *v { "true" } else { "false" }),
        Value::Number(v) => out.push_str(v),
        Value::String(v) => write_json_string(out, v),
        Value::List(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_value(out, item);
            }
            out.push(']');
        }
        Value::Map(map) => write_map_value(out, map),
    }
}

fn write_str_map(out: &mut String, map: &BTreeMap<String, String>) {
    out.push('{');
    for (i, (k, v)) in map.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        write_json_string(out, k);
        out.push(':');
        write_json_string(out, v);
    }
    out.push('}');
}

fn write_str_list(out: &mut String, items: &[String]) {
    out.push('[');
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        write_json_string(out, item);
    }
    out.push(']');
}

fn write_json_string(out: &mut String, input: &str) {
    out.push('"');
    escape_json(out, input);
    out.push('"');
}

fn escape_json(out: &mut String, input: &str) {
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
}

pub fn render_html(doc: &Document) -> String {
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
    for node in &doc.body {
        render_node(&mut out, node);
    }
    out.push_str("</html>");
    out
}

fn render_node(out: &mut String, node: &Node) {
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
        "paragraph" => wrap_inlines(out, "p", node),
        "section" => wrap_children(out, "section", node),
        "note" => wrap_children(out, "aside", node),
        "quote" => wrap_children(out, "blockquote", node),
        "list" => {
            let tag = if node.attrs.get("kind").map(|s| s.as_str()) == Some("ordered") {
                "ol"
            } else {
                "ul"
            };
            out.push_str(tag_open(tag, node).as_str());
            for child in &node.children {
                render_node(out, child);
            }
            out.push_str(&format!("</{}>", tag));
        }
        "item" => wrap_inlines(out, "li", node),
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
        "table" => wrap_children(out, "table", node),
        "row" => wrap_children(out, "tr", node),
        "cell" => {
            let tag = if node.attrs.get("header").map(|s| s.as_str()) == Some("true") {
                "th"
            } else {
                "td"
            };
            wrap_inlines(out, tag, node);
        }
        "figure" => wrap_children(out, "figure", node),
        "caption" => wrap_inlines(out, "figcaption", node),
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
        "form" => wrap_children(out, "dl", node),
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
            out.push_str(
                " aria-label=\"Table of contents\"><strong>Table of contents</strong></nav>",
            );
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
            for child in &node.children {
                render_node(out, child);
            }
            out.push_str("</figure>");
        }
        "bibliography" => wrap_children(out, "ol", node),
        "citation-entry" => wrap_inlines(out, "li", node),
        _ => wrap_children(out, "div", node),
    }
}

fn wrap_children(out: &mut String, tag: &str, node: &Node) {
    out.push_str(tag_open(tag, node).as_str());
    for child in &node.children {
        render_node(out, child);
    }
    out.push_str(&format!("</{}>", tag));
}

fn wrap_inlines(out: &mut String, tag: &str, node: &Node) {
    out.push_str(tag_open(tag, node).as_str());
    render_inlines(out, &node.inlines);
    for child in &node.children {
        render_node(out, child);
    }
    out.push_str(&format!("</{}>", tag));
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
            Inline::Span { children, .. } => {
                out.push_str("<span>");
                render_inlines(out, children);
                out.push_str("</span>");
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
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.starts_with('/') || trimmed.starts_with('#') {
        return false;
    }
    let scheme_end = trimmed.find(|c: char| !is_scheme_char(c));
    !matches!(scheme_end, Some(i) if i > 0 && trimmed[i..].starts_with(':'))
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

fn strip_forbidden_nods(input: &str) -> String {
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

pub fn render_tui(doc: &Document) -> String {
    let mut out = String::new();
    let ansi = ansi_enabled();
    if let Some(Value::String(title)) = doc.meta.get("title") {
        out.push_str(&paint(ansi, "1;36", title));
        out.push_str("\n");
        out.push_str(&paint(ansi, "2", &"═".repeat(title.chars().count().max(8))));
        out.push_str("\n\n");
    }
    for node in &doc.body {
        render_tui_node(&mut out, node, 0, ansi);
    }
    out
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

fn plain_node_text(node: &Node) -> String {
    let mut parts = Vec::new();
    if let Some(text) = &node.text {
        if !text.is_empty() {
            parts.push(text.clone());
        }
    }
    let inline = plain_inlines(&node.inlines);
    if !inline.is_empty() {
        parts.push(inline);
    }
    for child in &node.children {
        let child_text = plain_node_text(child);
        if !child_text.is_empty() {
            parts.push(child_text);
        }
    }
    parts.join(" ")
}

fn plain_inlines(inlines: &[Inline]) -> String {
    let mut out = String::new();
    for item in inlines {
        match item {
            Inline::Text(s) | Inline::Code(s) => out.push_str(s),
            Inline::Strong(c) | Inline::Em(c) => out.push_str(&plain_inlines(c)),
            Inline::Link { label, target } => {
                out.push_str(&plain_inlines(label));
                out.push_str(" <");
                out.push_str(target);
                out.push('>');
            }
            Inline::Span { children, .. } => out.push_str(&plain_inlines(children)),
            Inline::Var { namespace, name } => {
                out.push_str(&format!("{{{{{}.{}}}}}", namespace, name))
            }
            Inline::Ref { target } => out.push_str(&format!("@[{}]", target)),
            Inline::Mention { kind, target } => out.push_str(&format!("@{{{}:{}}}", kind, target)),
            Inline::FootnoteRef { target } => out.push_str(&format!("[^{}]", target)),
            Inline::CitationRef { target } => out.push_str(&format!("[@{}]", target)),
            Inline::MathInline { source } => out.push_str(source),
        }
    }
    out
}

pub fn ncp_json(doc: &Document) -> String {
    let mut out = String::from("{\"chunks\":[{\"id\":\"chunk-1\",\"nodes\":[");
    let ids = collect_node_ids(&doc.body);
    for (i, id) in ids.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        write_json_string(&mut out, id);
    }
    out.push_str("]}],\"loss\":[],\"mode\":\"semantic\",\"nodes\":");
    write_ncp_nodes(&mut out, &doc.body);
    out.push_str(",\"schema\":\"nodx-ncp/0.1\"}");
    out
}

fn collect_node_ids(nodes: &[Node]) -> Vec<String> {
    let mut out = Vec::new();
    for (i, node) in nodes.iter().enumerate() {
        out.push(node.id.clone().unwrap_or_else(|| format!("n{}", i + 1)));
    }
    out
}

fn write_ncp_nodes(out: &mut String, nodes: &[Node]) {
    out.push('[');
    for (i, node) in nodes.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push('[');
        write_json_string(out, &node.node_type);
        out.push(',');
        write_json_string(out, node.id.as_deref().unwrap_or(""));
        out.push(',');
        write_str_map(out, &node.attrs);
        out.push(',');
        write_json_string(
            out,
            node.text
                .as_deref()
                .unwrap_or(&plain_inlines(&node.inlines)),
        );
        out.push(']');
    }
    out.push(']');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_core_blocks() {
        let doc = parse_str("# Title {#t}\n\n:::note {type=\"warning\"}\nBody **x**.\n:::\n");
        assert_eq!(doc.body.len(), 2);
        assert_eq!(doc.body[0].node_type, "heading");
        assert_eq!(doc.body[1].node_type, "note");
        assert!(canonical_json(&doc).contains("\"type\":\"strong\""));
    }

    #[test]
    fn parses_tables_and_lists() {
        let doc = parse_str("- [ ] Todo\n- [x] Done\n\n| A | B |\n| - | - |\n| 1 | 2 |\n");
        assert_eq!(
            doc.body[0].attrs.get("kind").map(String::as_str),
            Some("task")
        );
        assert_eq!(doc.body[1].node_type, "table");
    }

    #[test]
    fn safe_link_url_blocks_dangerous_schemes() {
        assert_eq!(safe_link_url("https://example.com"), Some("https://example.com".into()));
        assert_eq!(safe_link_url("#anchor"), Some("#anchor".into()));
        assert_eq!(safe_link_url("relative/path"), Some("relative/path".into()));
        assert_eq!(safe_link_url("javascript:alert(1)"), None);
        assert_eq!(safe_link_url(" javascript:alert(1)"), None);
        assert_eq!(safe_link_url("JaVaScRiPt:alert(1)"), None);
        assert_eq!(safe_link_url("ja%76ascript:alert(1)"), None);
        assert_eq!(safe_link_url("data:text/html,<script>"), None);
        assert_eq!(safe_link_url("vbscript:alert(1)"), None);
        assert_eq!(safe_link_url("file:///etc/passwd"), None);
        assert_eq!(safe_link_url("\u{0008}javascript:alert"), None);
    }

    #[test]
    fn safe_image_url_allows_only_image_data_uris() {
        assert!(safe_image_url("https://example.com/x.png").is_some());
        assert!(safe_image_url("data:image/png;base64,AAA").is_some());
        assert!(safe_image_url("data:image/svg+xml,<svg/>").is_some());
        assert!(safe_image_url("data:text/html,<script>").is_none());
        assert!(safe_image_url("javascript:alert(1)").is_none());
    }

    #[test]
    fn html_render_blocks_javascript_link() {
        let doc = parse_str("[click](javascript:alert(1))\n");
        let html = render_html(&doc);
        assert!(!html.contains("href=\"javascript"));
        assert!(html.contains("nodx-blocked-link"));
    }

    #[test]
    fn unclosed_delimited_block_emits_diagnostic() {
        let doc = parse_str("::::section\nbody\n:::note\nx\n");
        assert!(doc
            .diagnostics
            .iter()
            .any(|d| d.code == "NODX-E005" && d.message.contains("Unclosed")));
    }

    #[test]
    fn front_matter_block_sequence_of_mappings() {
        let doc = parse_str("---\nschema: nodx/0.1\nauthors:\n  - name: Alice\n  - name: Bob\n---\n\nBody\n");
        let authors = doc.meta.get("authors").expect("authors");
        match authors {
            Value::List(items) => {
                assert_eq!(items.len(), 2);
                if let Value::Map(m) = &items[0] {
                    assert!(matches!(m.get("name"), Some(Value::String(s)) if s == "Alice"));
                } else {
                    panic!("expected first author to be a map");
                }
            }
            other => panic!("expected list, got {:?}", other),
        }
    }

    #[test]
    fn labelled_close_produces_same_ast_as_plain() {
        let plain = parse_str(":::note\nBody.\n:::\n");
        let labelled = parse_str(":::note\nBody.\n::: note\n");
        assert_eq!(canonical_json(&plain), canonical_json(&labelled));
        assert!(labelled
            .diagnostics
            .iter()
            .all(|d| d.code != "NODX-E005"));
    }

    #[test]
    fn labelled_close_mismatch_emits_diagnostic_but_recovers() {
        let doc = parse_str("::::section\n:::note\nBody.\n::: figure\n::::\n");
        let codes: Vec<_> = doc.diagnostics.iter().map(|d| d.code.as_str()).collect();
        assert!(codes.contains(&"NODX-E005"));
        assert_eq!(doc.body.len(), 1);
        assert_eq!(doc.body[0].node_type, "section");
    }

    #[test]
    fn style_block_is_literal_and_emits_style_tag() {
        let doc = parse_str(":::style\nh1 { color: red; }\n:::\n");
        assert_eq!(doc.body[0].node_type, "style");
        assert_eq!(doc.body[0].text.as_deref(), Some("h1 { color: red; }"));
        let html = render_html(&doc);
        assert!(html.contains("<style>h1 { color: red; }</style>"));
    }

    #[test]
    fn style_block_blocks_html_breakout() {
        let doc = parse_str(":::style\nbody { color: red; } </style><script>alert(1)</script>\n:::\n");
        let html = render_html(&doc);
        assert!(!html.to_lowercase().contains("<script"));
        assert!(html.contains("blocked unsafe style content"));
    }

    #[test]
    fn html_emits_lang_and_dir_on_root() {
        let doc = parse_str("---\nschema: nodx/0.1\nlanguage: ar\ndir: rtl\n---\n\n# T\n");
        let html = render_html(&doc);
        assert!(html.contains("<html lang=\"ar\" dir=\"rtl\">"));
    }

    #[test]
    fn forbidden_nods_emits_e027_and_strips_rule() {
        let doc = parse_str(":::style\na:hover { color: red; }\n.x { transform: scale(2); }\np { color: blue; }\n:::\n");
        let codes: Vec<_> = doc.diagnostics.iter().map(|d| d.code.as_str()).collect();
        assert!(codes.contains(&"NODX-E027"));
        let html = render_html(&doc);
        assert!(!html.contains(":hover"));
        assert!(!html.contains("transform"));
        assert!(html.contains("color: blue"));
        assert!(html.contains("forbidden NODS rule omitted"));
    }

    #[test]
    fn allowed_nods_passes_without_e027() {
        let doc = parse_str(":::style\nh1 { color: #0f766e; font-size: 24pt; }\n@page { size: A4 portrait; margin: 22mm; }\n:::\n");
        assert!(doc
            .diagnostics
            .iter()
            .all(|d| d.code != "NODX-E027"));
        let html = render_html(&doc);
        assert!(html.contains("color: #0f766e"));
        assert!(html.contains("@page"));
    }

    #[test]
    fn opener_is_not_confused_with_labelled_close() {
        let doc = parse_str(":::table\n:::row\n:::cell\nA\n:::\n:::\n:::\n");
        assert_eq!(doc.body.len(), 1);
        assert_eq!(doc.body[0].node_type, "table");
        assert_eq!(doc.body[0].children[0].node_type, "row");
        assert_eq!(doc.body[0].children[0].children[0].node_type, "cell");
    }

    #[test]
    fn package_rejects_duplicate_paths() {
        let bytes = build_zip_with_duplicate_path();
        let err = parse_bytes(&bytes).expect_err("should reject duplicate ZIP paths");
        assert!(err.message.contains("Duplicate"));
    }

    fn build_zip_with_duplicate_path() -> Vec<u8> {
        let mut out = Vec::new();
        let mut entries: Vec<(String, Vec<u8>, u32)> = Vec::new();
        for (name, data) in [
            ("mimetype", b"application/nodx+zip".to_vec()),
            ("manifest.yaml", b"schema: nodx-package/0.1\nentry: doc.nodx\n".to_vec()),
            ("doc.nodx", b"# A".to_vec()),
            ("doc.nodx", b"# B".to_vec()),
        ] {
            let local_offset = out.len() as u32;
            out.extend_from_slice(&[0x50, 0x4b, 0x03, 0x04]);
            out.extend_from_slice(&[20, 0]);
            out.extend_from_slice(&[0, 0]);
            out.extend_from_slice(&[0, 0]);
            out.extend_from_slice(&[0, 0, 0, 0]);
            out.extend_from_slice(&[0, 0, 0, 0]);
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&(name.len() as u16).to_le_bytes());
            out.extend_from_slice(&[0, 0]);
            out.extend_from_slice(name.as_bytes());
            out.extend_from_slice(&data);
            entries.push((name.to_string(), data, local_offset));
        }
        let cd_offset = out.len() as u32;
        for (name, data, local_offset) in &entries {
            out.extend_from_slice(&[0x50, 0x4b, 0x01, 0x02]);
            out.extend_from_slice(&[20, 0]);
            out.extend_from_slice(&[20, 0]);
            out.extend_from_slice(&[0, 0]);
            out.extend_from_slice(&[0, 0]);
            out.extend_from_slice(&[0, 0, 0, 0]);
            out.extend_from_slice(&[0, 0, 0, 0]);
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&(name.len() as u16).to_le_bytes());
            out.extend_from_slice(&[0, 0]);
            out.extend_from_slice(&[0, 0]);
            out.extend_from_slice(&[0, 0]);
            out.extend_from_slice(&[0, 0]);
            out.extend_from_slice(&[0, 0, 0, 0]);
            out.extend_from_slice(&local_offset.to_le_bytes());
            out.extend_from_slice(name.as_bytes());
        }
        let cd_size = (out.len() as u32) - cd_offset;
        out.extend_from_slice(&[0x50, 0x4b, 0x05, 0x06]);
        out.extend_from_slice(&[0, 0]);
        out.extend_from_slice(&[0, 0]);
        out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        out.extend_from_slice(&cd_size.to_le_bytes());
        out.extend_from_slice(&cd_offset.to_le_bytes());
        out.extend_from_slice(&[0, 0]);
        out
    }
}

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use nodx_core::{Document, Inline, Node, canonical_json, valid_name};
use nodx_validate::{Validator, exit_code_for};

const CHANGE_SCHEMA: &str = "nodx/change/1.1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Batch {
    pub author: Option<String>,
    pub time: Option<String>,
    pub reason: Option<String>,
    pub operations: Vec<Operation>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Operation {
    Insert {
        target: Target,
        position: InsertPosition,
        before_hash: String,
        node: Node,
    },
    Replace {
        target: Target,
        before_hash: String,
        node: Node,
    },
    Delete {
        target: Target,
        before_hash: String,
    },
    AddAttribute {
        target: Target,
        before_hash: String,
        name: String,
        value: String,
    },
    SetAttribute {
        target: Target,
        before_hash: String,
        name: String,
        value: String,
    },
    RemoveAttribute {
        target: Target,
        before_hash: String,
        name: String,
    },
    AddComment {
        target: Target,
        before_hash: String,
        author: Option<String>,
        text: String,
    },
    Approve {
        target: Target,
        before_hash: String,
        reviewer: Option<String>,
    },
    Reject {
        target: Target,
        before_hash: String,
        reviewer: Option<String>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Target {
    Id(String),
    Path(Vec<usize>),
    Hash(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InsertPosition {
    Before,
    After,
    AppendChild,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MutationReport {
    pub records: Vec<ChangeRecord>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChangeRecord {
    pub schema: String,
    pub op: String,
    pub target: String,
    pub path: String,
    pub before_hash: String,
    pub after_hash: String,
    pub author: Option<String>,
    pub time: Option<String>,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MutationError {
    pub code: MutationErrorCode,
    pub message: String,
    pub operation_index: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MutationErrorCode {
    AmbiguousTarget,
    HashMismatch,
    InvalidAttribute,
    InvalidOperation,
    MissingTarget,
    ValidationFailed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ResolvedTarget {
    path: Vec<usize>,
    hash: String,
}

impl Batch {
    pub fn new(operations: Vec<Operation>) -> Self {
        Self {
            author: None,
            time: None,
            reason: None,
            operations,
        }
    }
}

impl Target {
    pub fn id(id: &str) -> Self {
        Self::Id(id.to_string())
    }

    pub fn path(path: &[usize]) -> Self {
        Self::Path(path.to_vec())
    }

    pub fn hash(hash: &str) -> Self {
        Self::Hash(hash.to_string())
    }

    fn label(&self) -> String {
        match self {
            Target::Id(id) => format!("#{id}"),
            Target::Path(path) => path_to_string(path),
            Target::Hash(hash) => hash.clone(),
        }
    }
}

impl MutationReport {
    pub fn jsonl(&self) -> String {
        let mut out = String::new();
        for record in &self.records {
            write_record(&mut out, record);
            out.push('\n');
        }
        out
    }
}

impl ChangeRecord {
    fn new(
        batch: &Batch,
        op: &str,
        target: &Target,
        path: Vec<usize>,
        before_hash: String,
        after_hash: String,
    ) -> Self {
        Self {
            schema: CHANGE_SCHEMA.to_string(),
            op: op.to_string(),
            target: target.label(),
            path: path_to_string(&path),
            before_hash,
            after_hash,
            author: batch.author.clone(),
            time: batch.time.clone(),
            reason: batch.reason.clone(),
        }
    }
}

pub fn apply_batch(doc: &mut Document, batch: &Batch) -> Result<MutationReport, MutationError> {
    let mut working = doc.clone();
    let mut records = Vec::new();

    for (index, operation) in batch.operations.iter().enumerate() {
        let record = apply_operation(&mut working, batch, operation)
            .map_err(|err| err.with_operation(index))?;
        validate_document(&working).map_err(|err| err.with_operation(index))?;
        records.push(record);
    }

    *doc = working;
    Ok(MutationReport { records })
}

pub fn node_hash(node: &Node) -> String {
    nodx_package::sha256_base64url(node_hash_input(node).as_bytes())
}

pub fn document_hash(doc: &Document) -> String {
    nodx_package::sha256_base64url(canonical_json(doc).as_bytes())
}

fn apply_operation(
    doc: &mut Document,
    batch: &Batch,
    operation: &Operation,
) -> Result<ChangeRecord, MutationError> {
    match operation {
        Operation::Insert {
            target,
            position,
            before_hash,
            node,
        } => {
            let resolved = resolve_target(doc, target)?;
            require_hash(&resolved, before_hash)?;
            let inserted_path = insert_node(doc, &resolved.path, position, node.clone())?;
            let after_hash = node_hash(
                get_node(doc, &inserted_path)
                    .expect("inserted path exists immediately after insertion"),
            );
            Ok(ChangeRecord::new(
                batch,
                "insert",
                target,
                inserted_path,
                before_hash.clone(),
                after_hash,
            ))
        }
        Operation::Replace {
            target,
            before_hash,
            node,
        } => {
            let resolved = resolve_target(doc, target)?;
            require_hash(&resolved, before_hash)?;
            *get_node_mut(doc, &resolved.path).expect("resolved target path exists") = node.clone();
            let after_hash = node_hash(
                get_node(doc, &resolved.path).expect("replaced target path exists after replace"),
            );
            Ok(ChangeRecord::new(
                batch,
                "replace",
                target,
                resolved.path,
                before_hash.clone(),
                after_hash,
            ))
        }
        Operation::Delete {
            target,
            before_hash,
        } => {
            let resolved = resolve_target(doc, target)?;
            require_hash(&resolved, before_hash)?;
            remove_node(doc, &resolved.path)?;
            let after_hash = document_hash(doc);
            Ok(ChangeRecord::new(
                batch,
                "delete",
                target,
                resolved.path,
                before_hash.clone(),
                after_hash,
            ))
        }
        Operation::AddAttribute {
            target,
            before_hash,
            name,
            value,
        } => mutate_attribute(doc, batch, target, before_hash, "add-attribute", |node| {
            validate_attr_name(name)?;
            if node.attrs.contains_key(name) {
                return Err(err(
                    MutationErrorCode::InvalidOperation,
                    "Attribute already exists.",
                ));
            }
            node.attrs.insert(name.clone(), value.clone());
            Ok(())
        }),
        Operation::SetAttribute {
            target,
            before_hash,
            name,
            value,
        } => mutate_attribute(doc, batch, target, before_hash, "set-attribute", |node| {
            validate_attr_name(name)?;
            node.attrs.insert(name.clone(), value.clone());
            Ok(())
        }),
        Operation::RemoveAttribute {
            target,
            before_hash,
            name,
        } => mutate_attribute(
            doc,
            batch,
            target,
            before_hash,
            "remove-attribute",
            |node| {
                validate_attr_name(name)?;
                if node.attrs.remove(name).is_none() {
                    return Err(err(
                        MutationErrorCode::InvalidOperation,
                        "Attribute does not exist.",
                    ));
                }
                Ok(())
            },
        ),
        Operation::AddComment {
            target,
            before_hash,
            author,
            text,
        } => mutate_attribute(doc, batch, target, before_hash, "add-comment", |node| {
            let mut attrs = BTreeMap::new();
            if let Some(author) = author {
                attrs.insert("author".to_string(), author.clone());
            }
            node.children.push(Node {
                node_type: "comment".to_string(),
                id: None,
                classes: Vec::new(),
                attrs,
                children: Vec::new(),
                inlines: vec![Inline::Text(text.clone())],
                text: None,
            });
            Ok(())
        }),
        Operation::Approve {
            target,
            before_hash,
            reviewer,
        } => review_state(
            doc,
            batch,
            target,
            before_hash,
            reviewer,
            "approve",
            "approved",
        ),
        Operation::Reject {
            target,
            before_hash,
            reviewer,
        } => review_state(
            doc,
            batch,
            target,
            before_hash,
            reviewer,
            "reject",
            "rejected",
        ),
    }
}

fn mutate_attribute<F>(
    doc: &mut Document,
    batch: &Batch,
    target: &Target,
    before_hash: &str,
    op: &str,
    mutate: F,
) -> Result<ChangeRecord, MutationError>
where
    F: FnOnce(&mut Node) -> Result<(), MutationError>,
{
    let resolved = resolve_target(doc, target)?;
    require_hash(&resolved, before_hash)?;
    mutate(get_node_mut(doc, &resolved.path).expect("resolved target path exists"))?;
    let after_hash =
        node_hash(get_node(doc, &resolved.path).expect("mutated target path exists after op"));
    Ok(ChangeRecord::new(
        batch,
        op,
        target,
        resolved.path,
        before_hash.to_string(),
        after_hash,
    ))
}

fn review_state(
    doc: &mut Document,
    batch: &Batch,
    target: &Target,
    before_hash: &str,
    reviewer: &Option<String>,
    op: &str,
    status: &str,
) -> Result<ChangeRecord, MutationError> {
    mutate_attribute(doc, batch, target, before_hash, op, |node| {
        node.attrs.insert("status".to_string(), status.to_string());
        if let Some(reviewer) = reviewer {
            node.attrs
                .insert("reviewed-by".to_string(), reviewer.to_string());
        }
        Ok(())
    })
}

fn resolve_target(doc: &Document, target: &Target) -> Result<ResolvedTarget, MutationError> {
    let mut matches = Vec::new();
    collect_matches(&doc.body, target, &mut Vec::new(), &mut matches);
    match matches.len() {
        0 => Err(err(
            MutationErrorCode::MissingTarget,
            "Mutation target was not found.",
        )),
        1 => Ok(matches.remove(0)),
        _ => Err(err(
            MutationErrorCode::AmbiguousTarget,
            "Mutation target resolves to multiple nodes.",
        )),
    }
}

fn collect_matches(
    nodes: &[Node],
    target: &Target,
    path: &mut Vec<usize>,
    matches: &mut Vec<ResolvedTarget>,
) {
    for (index, node) in nodes.iter().enumerate() {
        path.push(index);
        let hash = node_hash(node);
        let found = match target {
            Target::Id(id) => node.id.as_deref() == Some(id.as_str()),
            Target::Path(target_path) => path == target_path,
            Target::Hash(target_hash) => &hash == target_hash,
        };
        if found {
            matches.push(ResolvedTarget {
                path: path.clone(),
                hash: hash.clone(),
            });
        }
        collect_matches(&node.children, target, path, matches);
        path.pop();
    }
}

fn require_hash(resolved: &ResolvedTarget, before_hash: &str) -> Result<(), MutationError> {
    if before_hash != resolved.hash {
        return Err(err(
            MutationErrorCode::HashMismatch,
            "beforeHash does not match the resolved target.",
        ));
    }
    Ok(())
}

fn insert_node(
    doc: &mut Document,
    target_path: &[usize],
    position: &InsertPosition,
    node: Node,
) -> Result<Vec<usize>, MutationError> {
    match position {
        InsertPosition::AppendChild => {
            let parent = get_node_mut(doc, target_path).expect("resolved target path exists");
            parent.children.push(node);
            let mut inserted = target_path.to_vec();
            inserted.push(parent.children.len() - 1);
            Ok(inserted)
        }
        InsertPosition::Before | InsertPosition::After => {
            let (parent_path, index) = split_parent(target_path)?;
            let parent = child_vec_mut(doc, parent_path)?;
            let insert_at = match position {
                InsertPosition::Before => index,
                InsertPosition::After => index + 1,
                InsertPosition::AppendChild => unreachable!("handled above"),
            };
            parent.insert(insert_at, node);
            let mut inserted = parent_path.to_vec();
            inserted.push(insert_at);
            Ok(inserted)
        }
    }
}

fn remove_node(doc: &mut Document, path: &[usize]) -> Result<Node, MutationError> {
    let (parent_path, index) = split_parent(path)?;
    let parent = child_vec_mut(doc, parent_path)?;
    if index >= parent.len() {
        return Err(err(
            MutationErrorCode::MissingTarget,
            "Mutation target path is out of bounds.",
        ));
    }
    Ok(parent.remove(index))
}

fn split_parent(path: &[usize]) -> Result<(&[usize], usize), MutationError> {
    path.split_last()
        .map(|(index, parent)| (parent, *index))
        .ok_or_else(|| {
            err(
                MutationErrorCode::MissingTarget,
                "Mutation target path is empty.",
            )
        })
}

fn child_vec_mut<'a>(
    doc: &'a mut Document,
    parent_path: &[usize],
) -> Result<&'a mut Vec<Node>, MutationError> {
    if parent_path.is_empty() {
        return Ok(&mut doc.body);
    }
    let parent = get_node_mut(doc, parent_path).ok_or_else(|| {
        err(
            MutationErrorCode::MissingTarget,
            "Mutation parent path was not found.",
        )
    })?;
    Ok(&mut parent.children)
}

fn get_node<'a>(doc: &'a Document, path: &[usize]) -> Option<&'a Node> {
    let (first, rest) = path.split_first()?;
    let mut node = doc.body.get(*first)?;
    for index in rest {
        node = node.children.get(*index)?;
    }
    Some(node)
}

fn get_node_mut<'a>(doc: &'a mut Document, path: &[usize]) -> Option<&'a mut Node> {
    let (first, rest) = path.split_first()?;
    let mut node = doc.body.get_mut(*first)?;
    for index in rest {
        node = node.children.get_mut(*index)?;
    }
    Some(node)
}

fn validate_attr_name(name: &str) -> Result<(), MutationError> {
    if name == "id" || name == "class" || name == "classes" || !valid_name(name, false) {
        return Err(err(
            MutationErrorCode::InvalidAttribute,
            "Attribute name is invalid or reserved.",
        ));
    }
    Ok(())
}

fn validate_document(doc: &Document) -> Result<(), MutationError> {
    let diagnostics = Validator::default().validate(doc);
    if exit_code_for(&diagnostics) != 0 {
        let message = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.severity == "fatal" || diagnostic.severity == "error")
            .map(|diagnostic| diagnostic.message.clone())
            .unwrap_or_else(|| "Mutation produced an invalid document.".to_string());
        return Err(err(MutationErrorCode::ValidationFailed, &message));
    }
    Ok(())
}

fn node_hash_input(node: &Node) -> String {
    let mut out = String::new();
    out.push_str(&node.node_type);
    out.push('\n');
    if let Some(id) = &node.id {
        out.push_str(id);
    }
    out.push('\n');
    write_str_map(&mut out, &node.attrs);
    out.push('\n');
    out.push_str(node.text.as_deref().unwrap_or(""));
    out.push_str(&plain_inlines(&node.inlines));
    for child in &node.children {
        out.push('\n');
        out.push_str(&node_hash_input(child));
    }
    out
}

fn plain_inlines(inlines: &[Inline]) -> String {
    let mut out = String::new();
    for inline in inlines {
        match inline {
            Inline::Text(text)
            | Inline::Code(text)
            | Inline::MathInline { source: text }
            | Inline::Var {
                namespace: _,
                name: text,
            }
            | Inline::Ref { target: text }
            | Inline::Mention {
                kind: _,
                target: text,
            }
            | Inline::FootnoteRef { target: text }
            | Inline::CitationRef { target: text } => out.push_str(text),
            Inline::Strong(children)
            | Inline::Em(children)
            | Inline::Mark(children)
            | Inline::Sub(children)
            | Inline::Sup(children)
            | Inline::Span { children, attrs: _ } => out.push_str(&plain_inlines(children)),
            Inline::Link { label, target: _ } => out.push_str(&plain_inlines(label)),
        }
    }
    out
}

fn path_to_string(path: &[usize]) -> String {
    path.iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(".")
}

fn write_record(out: &mut String, record: &ChangeRecord) {
    out.push_str("{\"afterHash\":");
    write_json_string(out, &record.after_hash);
    write_option_string(out, ",\"author\":", record.author.as_deref());
    out.push_str(",\"beforeHash\":");
    write_json_string(out, &record.before_hash);
    out.push_str(",\"op\":");
    write_json_string(out, &record.op);
    out.push_str(",\"path\":");
    write_json_string(out, &record.path);
    write_option_string(out, ",\"reason\":", record.reason.as_deref());
    out.push_str(",\"schema\":");
    write_json_string(out, &record.schema);
    out.push_str(",\"target\":");
    write_json_string(out, &record.target);
    write_option_string(out, ",\"time\":", record.time.as_deref());
    out.push('}');
}

fn write_option_string(out: &mut String, name: &str, value: Option<&str>) {
    if let Some(value) = value {
        out.push_str(name);
        write_json_string(out, value);
    }
}

fn write_str_map(out: &mut String, map: &BTreeMap<String, String>) {
    out.push('{');
    for (i, (key, value)) in map.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        write_json_string(out, key);
        out.push(':');
        write_json_string(out, value);
    }
    out.push('}');
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

fn err(code: MutationErrorCode, message: &str) -> MutationError {
    MutationError {
        code,
        message: message.to_string(),
        operation_index: 0,
    }
}

impl MutationError {
    fn with_operation(mut self, operation_index: usize) -> Self {
        self.operation_index = operation_index;
        self
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use nodx_core::{Inline, Node, parse_str};

    use super::*;

    fn paragraph(id: &str, text: &str) -> Node {
        Node {
            node_type: "paragraph".to_string(),
            id: Some(id.to_string()),
            classes: Vec::new(),
            attrs: BTreeMap::new(),
            children: Vec::new(),
            inlines: vec![Inline::Text(text.to_string())],
            text: None,
        }
    }

    fn fixture_doc() -> Document {
        parse_str(
            "---\nschema: nodx/1.0\n---\n# Title {#title}\n\n:::note {#note fallback=\"children\"}\nReview me.\n:::\n",
        )
    }

    #[test]
    fn hash_targets_match_ncp_node_hashes() {
        let doc = fixture_doc();
        let note = get_node(&doc, &[1]).unwrap();
        assert_eq!(
            node_hash(note),
            "sha256-MCssPUYzhZQUlFi1HaE-r-NCBlncksjtTtUF914xzNA"
        );
    }

    #[test]
    fn emits_fixture_backed_jsonl_change_records() {
        let mut doc = fixture_doc();
        let before = node_hash(get_node(&doc, &[1]).unwrap());
        let batch = Batch {
            author: Some("agent:test".to_string()),
            time: Some("2026-05-11T00:00:00Z".to_string()),
            reason: Some("fixture".to_string()),
            operations: vec![
                Operation::SetAttribute {
                    target: Target::id("note"),
                    before_hash: before,
                    name: "status".to_string(),
                    value: "approved".to_string(),
                },
                Operation::AddComment {
                    target: Target::id("note"),
                    before_hash: "sha256-pcaCDe--e93W_XnlB1B-tjdJKojU-qJabiGP-Ej0Gck".to_string(),
                    author: Some("agent:test".to_string()),
                    text: "Looks consistent.".to_string(),
                },
            ],
        };

        let report = apply_batch(&mut doc, &batch).unwrap();

        assert_eq!(
            report.jsonl(),
            include_str!("../tests/fixtures/change_records.jsonl")
        );
    }

    #[test]
    fn hash_mismatch_rolls_back_batch() {
        let mut doc = fixture_doc();
        let original = doc.clone();
        let batch = Batch::new(vec![Operation::SetAttribute {
            target: Target::id("note"),
            before_hash: "sha256-wrong".to_string(),
            name: "status".to_string(),
            value: "approved".to_string(),
        }]);

        let err = apply_batch(&mut doc, &batch).unwrap_err();

        assert_eq!(err.code, MutationErrorCode::HashMismatch);
        assert_eq!(doc, original);
    }

    #[test]
    fn validation_failure_rolls_back_full_batch() {
        let mut doc = fixture_doc();
        let original = doc.clone();
        let title_hash = node_hash(get_node(&doc, &[0]).unwrap());
        let note_hash = node_hash(get_node(&doc, &[1]).unwrap());
        let batch = Batch::new(vec![
            Operation::SetAttribute {
                target: Target::id("note"),
                before_hash: note_hash,
                name: "status".to_string(),
                value: "approved".to_string(),
            },
            Operation::Replace {
                target: Target::id("title"),
                before_hash: title_hash,
                node: {
                    let mut node = paragraph("title", "");
                    node.node_type = "image".to_string();
                    node
                },
            },
        ]);

        let err = apply_batch(&mut doc, &batch).unwrap_err();

        assert_eq!(err.code, MutationErrorCode::ValidationFailed);
        assert_eq!(err.operation_index, 1);
        assert_eq!(doc, original);
    }

    #[test]
    fn resolves_targets_by_id_path_and_hash() {
        let mut doc = fixture_doc();
        let title_hash = node_hash(get_node(&doc, &[0]).unwrap());
        let note_hash = node_hash(get_node(&doc, &[1]).unwrap());
        let batch = Batch::new(vec![
            Operation::Approve {
                target: Target::hash(&note_hash),
                before_hash: note_hash,
                reviewer: Some("agent:test".to_string()),
            },
            Operation::Insert {
                target: Target::path(&[0]),
                position: InsertPosition::After,
                before_hash: title_hash,
                node: paragraph("intro", "Inserted."),
            },
        ]);

        apply_batch(&mut doc, &batch).unwrap();

        assert_eq!(get_node(&doc, &[1]).unwrap().id.as_deref(), Some("intro"));
        assert_eq!(
            get_node(&doc, &[2])
                .unwrap()
                .attrs
                .get("status")
                .map(String::as_str),
            Some("approved")
        );
    }

    #[test]
    fn supports_delete_and_remove_attribute() {
        let mut doc = fixture_doc();
        let note_hash = node_hash(get_node(&doc, &[1]).unwrap());
        let batch = Batch::new(vec![
            Operation::SetAttribute {
                target: Target::id("note"),
                before_hash: note_hash,
                name: "status".to_string(),
                value: "approved".to_string(),
            },
            Operation::RemoveAttribute {
                target: Target::id("note"),
                before_hash: "sha256-pcaCDe--e93W_XnlB1B-tjdJKojU-qJabiGP-Ej0Gck".to_string(),
                name: "status".to_string(),
            },
            Operation::Delete {
                target: Target::id("note"),
                before_hash: "sha256-MCssPUYzhZQUlFi1HaE-r-NCBlncksjtTtUF914xzNA".to_string(),
            },
        ]);

        apply_batch(&mut doc, &batch).unwrap();

        assert!(resolve_target(&doc, &Target::id("note")).is_err());
    }

    #[test]
    fn supports_add_attribute_replace_and_reject() {
        let mut doc = fixture_doc();
        let note_hash = node_hash(get_node(&doc, &[1]).unwrap());
        apply_batch(
            &mut doc,
            &Batch::new(vec![Operation::AddAttribute {
                target: Target::id("note"),
                before_hash: note_hash,
                name: "priority".to_string(),
                value: "high".to_string(),
            }]),
        )
        .unwrap();
        let note_hash = node_hash(get_node(&doc, &[1]).unwrap());
        apply_batch(
            &mut doc,
            &Batch::new(vec![Operation::Reject {
                target: Target::id("note"),
                before_hash: note_hash,
                reviewer: Some("agent:test".to_string()),
            }]),
        )
        .unwrap();
        let note_hash = node_hash(get_node(&doc, &[1]).unwrap());
        apply_batch(
            &mut doc,
            &Batch::new(vec![Operation::Replace {
                target: Target::id("note"),
                before_hash: note_hash,
                node: paragraph("note", "Replacement."),
            }]),
        )
        .unwrap();

        let note = get_node(&doc, &[1]).unwrap();
        assert_eq!(plain_inlines(&note.inlines), "Replacement.");
        assert!(note.attrs.is_empty());
    }
}

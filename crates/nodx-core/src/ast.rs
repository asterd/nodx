use std::collections::BTreeMap;

use crate::diagnostic::Diagnostic;

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
    Mark(Vec<Inline>),
    Sub(Vec<Inline>),
    Sup(Vec<Inline>),
    Code(String),
    Link {
        label: Vec<Inline>,
        target: String,
        attrs: Attrs,
    },
    Span { children: Vec<Inline>, attrs: Attrs },
    Var { namespace: String, name: String },
    Ref { target: String },
    Mention { kind: String, target: String },
    FootnoteRef { target: String },
    CitationRef { target: String },
    MathInline { source: String },
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
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

impl Node {
    pub(crate) fn textual(node_type: &str, attrs: Attrs, inlines: Vec<Inline>) -> Self {
        Self::base(node_type, attrs, Vec::new(), inlines, None)
    }

    pub(crate) fn literal(node_type: &str, attrs: Attrs, text: String) -> Self {
        Self::base(node_type, attrs, Vec::new(), Vec::new(), Some(text))
    }

    pub(crate) fn container(node_type: &str, attrs: Attrs, children: Vec<Node>) -> Self {
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

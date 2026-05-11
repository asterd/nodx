#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub code: String,
    pub severity: String,
    pub message: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub target: Option<String>,
}

pub(crate) fn diag(
    code: &str,
    severity: &str,
    message: &str,
    line: usize,
    column: usize,
) -> Diagnostic {
    Diagnostic {
        code: code.to_string(),
        severity: severity.to_string(),
        message: message.to_string(),
        line: Some(line),
        column: Some(column),
        target: None,
    }
}

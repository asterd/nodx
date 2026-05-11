use crate::ast::Document;
use crate::diagnostic::Diagnostic;
use crate::limits::ResourceLimits;

pub fn parse_bytes(input: &[u8]) -> Result<Document, Diagnostic> {
    parse_bytes_with_limits(input, ResourceLimits::default())
}

pub fn parse_bytes_with_limits(
    input: &[u8],
    limits: ResourceLimits,
) -> Result<Document, Diagnostic> {
    if input.len() > limits.source_bytes {
        return Err(Diagnostic {
            code: "NODX-E012".to_string(),
            severity: "fatal".to_string(),
            message: "Input byte size limit exceeded.".to_string(),
            line: Some(1),
            column: Some(1),
            target: None,
        });
    }
    if is_packaged_nodx(input) {
        return Err(Diagnostic {
            code: "NODX-E001".to_string(),
            severity: "fatal".to_string(),
            message:
                "Packaged NODX inputs must be opened through the package reader before parsing."
                    .to_string(),
            line: Some(1),
            column: Some(1),
            target: None,
        });
    }
    match std::str::from_utf8(input) {
        Ok(s) => Ok(crate::block_parser::parse_str_with_limits(s, limits)),
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

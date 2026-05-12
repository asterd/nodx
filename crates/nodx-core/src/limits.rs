/// Resource caps applied during parsing and downstream processing.
///
/// Each field documents whether the limit is currently *enforced* by the
/// reference implementation, or *reserved* for features not yet shipped.
/// Reserved fields are kept on the struct so that callers writing forward
/// compatible code do not need to migrate when those features land.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceLimits {
    /// Maximum total input size in bytes (enforced by `parse_str_with_limits`).
    pub source_bytes: usize,
    /// Maximum size of the YAML front matter region (enforced by the parser).
    pub front_matter_bytes: usize,
    /// Maximum length of a single source line (enforced by the parser).
    pub line_length: usize,
    /// Cap on the byte length of an attribute value (enforced by `parse_attrs`;
    /// values exceeding the cap are silently dropped to avoid amplifying
    /// hostile input through the AST).
    pub attribute_value_bytes: usize,
    /// Cap on the byte length of a heading or block id (enforced by the
    /// validator; offending ids surface as diagnostics).
    pub id_bytes: usize,
    /// Maximum nesting depth of delimited blocks (enforced by the parser;
    /// exceeding the cap raises a single NODX-E012 diagnostic and the parser
    /// keeps recovering from sibling content).
    pub block_nesting_depth: usize,
    /// Reserved. Inline parsing is currently iterative; this cap will be
    /// enforced once nested inline constructs ship.
    pub inline_nesting_depth: usize,
    /// Maximum number of nodes the parser is allowed to emit (enforced by the
    /// parser; once reached, further nodes are skipped and a NODX-E012
    /// diagnostic is recorded).
    pub nodes_per_document: usize,
    /// Cap on the byte length of an inline `data:` URI payload (enforced by
    /// `nodx-url`).
    pub data_uri_bytes: usize,
    /// Reserved. Soft target for future expansion-aware passes; the canonical
    /// AST size is currently bounded transitively by `source_bytes` and
    /// `nodes_per_document`.
    pub expanded_ast_bytes: usize,
    /// Reserved for a future `::include` extension that may resolve nested
    /// documents.
    pub include_depth: usize,
    /// Cap on the total uncompressed size of a `.nodx` package (enforced by
    /// `nodx-package`).
    pub package_uncompressed_bytes: usize,
    /// Cap on the number of entries inside a `.nodx` package.
    pub package_file_count: usize,
    /// Cap on the uncompressed size of a single package entry.
    pub package_entry_bytes: usize,
    /// Maximum compression ratio (uncompressed / compressed) of a package
    /// entry. Used to detect zip bombs.
    pub package_compression_ratio: usize,
    /// Reserved. Currently the package reader rejects any nested ZIP entry
    /// outright (depth 0); the field is kept for forward compatibility.
    pub package_nested_zip_depth: usize,
    /// Maximum byte length of a path inside a package.
    pub package_path_bytes: usize,
    /// Maximum number of path segments inside a package.
    pub package_path_segments: usize,
    /// Cap on the byte length of a URL or path argument (enforced by
    /// `nodx-url`).
    pub url_bytes: usize,
    /// Maximum number of explicit manifest entries.
    pub manifest_entries: usize,
    /// Cap on the byte length of a JWS header (enforced by `nodx-sign`).
    pub signature_header_bytes: usize,
    /// Cap on the byte size of a single export artifact (enforced by
    /// `nodx-export`).
    pub export_bytes: usize,
    /// Cap on the number of files inside an export bundle.
    pub export_entry_count: usize,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            source_bytes: 64 * 1024 * 1024,
            front_matter_bytes: 64 * 1024,
            line_length: 1024 * 1024,
            attribute_value_bytes: 64 * 1024,
            id_bytes: 256,
            block_nesting_depth: 32,
            inline_nesting_depth: 32,
            nodes_per_document: 100_000,
            data_uri_bytes: 5 * 1024 * 1024,
            expanded_ast_bytes: 64 * 1024 * 1024,
            include_depth: 8,
            package_uncompressed_bytes: 256 * 1024 * 1024,
            package_file_count: 1_024,
            package_entry_bytes: 64 * 1024 * 1024,
            package_compression_ratio: 100,
            package_nested_zip_depth: 0,
            package_path_bytes: 512,
            package_path_segments: 8,
            url_bytes: 4 * 1024,
            manifest_entries: 1_024,
            signature_header_bytes: 8 * 1024,
            export_bytes: 256 * 1024 * 1024,
            export_entry_count: 1_024,
        }
    }
}

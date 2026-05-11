#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceLimits {
    pub source_bytes: usize,
    pub front_matter_bytes: usize,
    pub line_length: usize,
    pub attribute_value_bytes: usize,
    pub id_bytes: usize,
    pub block_nesting_depth: usize,
    pub inline_nesting_depth: usize,
    pub nodes_per_document: usize,
    pub data_uri_bytes: usize,
    pub expanded_ast_bytes: usize,
    pub include_depth: usize,
    pub package_uncompressed_bytes: usize,
    pub package_file_count: usize,
    pub package_entry_bytes: usize,
    pub package_compression_ratio: usize,
    pub package_nested_zip_depth: usize,
    pub url_bytes: usize,
    pub manifest_entries: usize,
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
            url_bytes: 4 * 1024,
            manifest_entries: 1_024,
        }
    }
}

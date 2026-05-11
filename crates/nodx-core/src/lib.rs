#![forbid(unsafe_code)]

mod ast;
mod attrs;
mod block_parser;
mod bytes;
pub mod canonical;
mod diagnostic;
mod front_matter;
mod hashing;
mod inline_parser;
mod limits;
mod navigation;
mod tui;

pub use ast::{Attrs, Document, Inline, Node, Value};
pub use attrs::valid_name;
pub use block_parser::{parse_str, parse_str_with_limits};
pub use bytes::{is_packaged_nodx, parse_bytes, parse_bytes_with_limits};
pub use canonical::canonical_json;
pub use diagnostic::Diagnostic;
pub use hashing::{base64url_decode, base64url_encode, crc32, sha256_base64url, sha256_bytes};
pub use inline_parser::{parse_inlines, plain_inlines, plain_node_text};
pub use limits::ResourceLimits;
pub use navigation::{
    NavigationEntry, NavigationGraph, ResolvedNavigation, default_navigation_label,
    resolve_navigation,
};
pub use tui::render_tui;

#[cfg(test)]
mod tests;

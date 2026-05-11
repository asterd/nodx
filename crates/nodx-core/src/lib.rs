#![forbid(unsafe_code)]

mod ast;
mod attrs;
mod block_parser;
mod bytes;
mod canonical;
mod diagnostic;
mod front_matter;
mod html_baseline;
mod inline_parser;
mod navigation;
mod ncp_baseline;
mod style_baseline;
mod tui;

pub use ast::{Attrs, Document, Inline, Node, Value};
pub use attrs::valid_name;
pub use block_parser::{parse_str, parse_str_with_limits};
pub use bytes::{is_packaged_nodx, parse_bytes, parse_bytes_with_limits};
pub use canonical::canonical_json;
pub use diagnostic::Diagnostic;
pub use html_baseline::{
    is_safe_asset_ref, render_html, render_html_with_limits, safe_image_url, safe_link_url,
};
pub use inline_parser::parse_inlines;
pub use navigation::{
    NavigationEntry, NavigationGraph, ResolvedNavigation, default_navigation_label,
    resolve_navigation,
};
pub use ncp_baseline::ncp_json;
pub use nodx_url::ResourceLimits;
pub use tui::render_tui;

#[cfg(test)]
mod tests;

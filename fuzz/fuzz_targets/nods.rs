#![no_main]

use libfuzzer_sys::fuzz_target;
use nodx_style::{audit_stylesheet, sanitize_stylesheet, style_urls};
use nodx_url::ResourceLimits;

fuzz_target!(|data: &str| {
    let limits = ResourceLimits::default();
    let _ = audit_stylesheet(data, limits);
    let _ = sanitize_stylesheet(data, limits);
    let _ = style_urls(data);
});

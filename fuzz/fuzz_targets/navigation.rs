#![no_main]

use libfuzzer_sys::fuzz_target;
use nodx_core::{ResourceLimits, parse_str_with_limits, resolve_navigation};

fuzz_target!(|data: &str| {
    let doc = parse_str_with_limits(data, ResourceLimits::default());
    let _ = resolve_navigation(&doc);
});

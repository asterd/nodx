#![no_main]

use libfuzzer_sys::fuzz_target;
use nodx_core::{ResourceLimits, ncp_json, parse_str_with_limits};

fuzz_target!(|data: &str| {
    let doc = parse_str_with_limits(data, ResourceLimits::default());
    let _ = ncp_json(&doc);
});

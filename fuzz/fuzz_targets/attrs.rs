#![no_main]

use libfuzzer_sys::fuzz_target;
use nodx_core::{ResourceLimits, parse_str_with_limits, valid_name};

fuzz_target!(|data: &str| {
    let _ = valid_name(data, true);
    let source = format!("# Heading {{{data}}}\n\n:::block {{{data}}}\n:::\n");
    let _ = parse_str_with_limits(&source, ResourceLimits::default());
});

#![no_main]

use libfuzzer_sys::fuzz_target;
use nodx_core::{ResourceLimits, parse_str_with_limits};

fuzz_target!(|data: &str| {
    let _ = parse_str_with_limits(data, ResourceLimits::default());
});

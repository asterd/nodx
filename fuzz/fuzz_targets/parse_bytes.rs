#![no_main]

use libfuzzer_sys::fuzz_target;
use nodx_core::{ResourceLimits, parse_bytes_with_limits};

fuzz_target!(|data: &[u8]| {
    let _ = parse_bytes_with_limits(data, ResourceLimits::default());
});

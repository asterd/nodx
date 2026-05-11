#![no_main]

use libfuzzer_sys::fuzz_target;
use nodx_core::{ResourceLimits, parse_bytes_with_limits};

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = std::str::from_utf8(data) {
        let source = format!("---\n{text}\n---\n\nBody\n");
        let _ = parse_bytes_with_limits(source.as_bytes(), ResourceLimits::default());
    }
});

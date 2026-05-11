#![no_main]

use libfuzzer_sys::fuzz_target;
use nodx_core::parse_inlines;

fuzz_target!(|data: &str| {
    let _ = parse_inlines(data);
});

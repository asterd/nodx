#![no_main]

use libfuzzer_sys::fuzz_target;
use nodx_package::Package;
use nodx_url::ResourceLimits;

fuzz_target!(|data: &[u8]| {
    let _ = Package::open(data, ResourceLimits::default());
});

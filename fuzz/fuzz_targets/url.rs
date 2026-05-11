#![no_main]

use libfuzzer_sys::fuzz_target;
use nodx_url::{ReferenceKind, ResourceLimits, ResourcePolicy, normalize_package_path};

fuzz_target!(|data: &str| {
    let policy = ResourcePolicy::default();
    let _ = policy.classify_uri(ReferenceKind::Link, data);
    let _ = policy.classify_uri(ReferenceKind::Asset, data);
    let _ = normalize_package_path(data, ResourceLimits::default());
});

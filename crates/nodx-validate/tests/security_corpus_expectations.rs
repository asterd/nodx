//! Security corpus is gated. For each `.nodx` fixture under
//! `spec/tests/security/{yaml-hostile,nods-hostile,xss}/`, the matching
//! `expectations.json` file lists:
//!   - `codes`: the set of diagnostic codes that MUST be emitted, and
//!   - `exit_code`: the exit code the CLI MUST return.
//!
//! Together these fields are the *contract* for hostile-input handling.
//! A regression that silently accepts a `javascript:` URL, an unsafe NODS
//! construct, or a forbidden YAML form would flip this test red.
//!
//! The fixture-to-expectation mapping is byte-exact: every fixture must
//! have an entry, every entry must point to a fixture. There is no
//! lenient mode.
//!
//! When adding a fixture, regenerate the expectations with the helper
//! described in `spec/tests/security/README.md`, or by running the same
//! pipeline this test runs and committing the result.

use std::{collections::BTreeMap, fs, path::Path};

use nodx_core::{Diagnostic, ResourceLimits, is_packaged_nodx, parse_bytes_with_limits};
use nodx_validate::Validator;

#[derive(Debug, serde::Deserialize)]
struct Expectation {
    codes: Vec<String>,
    exit_code: i32,
}

const CATEGORIES: &[&str] = &["yaml-hostile", "nods-hostile", "xss"];

#[test]
fn security_corpus_matches_expectations() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut failures: Vec<String> = Vec::new();

    for category in CATEGORIES {
        let dir = root.join("spec/tests/security").join(category);
        let expectations_path = dir.join("expectations.json");
        let expectations_bytes = fs::read(&expectations_path).unwrap_or_else(|err| {
            panic!(
                "missing expectations file {}: {err}",
                expectations_path.display()
            )
        });
        let expectations: BTreeMap<String, Expectation> =
            serde_json::from_slice(&expectations_bytes)
                .unwrap_or_else(|err| panic!("malformed {}: {err}", expectations_path.display()));

        let mut seen: BTreeMap<String, bool> =
            expectations.keys().map(|k| (k.clone(), false)).collect();

        for entry in fs::read_dir(&dir).expect("read security category dir") {
            let entry = entry.expect("dirent");
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if !name.ends_with(".nodx") {
                continue;
            }

            let Some(expected) = expectations.get(name) else {
                failures.push(format!(
                    "{category}/{name}: fixture exists but no expectations entry"
                ));
                continue;
            };
            seen.insert(name.to_string(), true);

            let (actual_codes, actual_exit) = run_pipeline(&path);
            let expected_codes: Vec<String> = {
                let mut v = expected.codes.clone();
                v.sort();
                v.dedup();
                v
            };

            if actual_codes != expected_codes {
                failures.push(format!(
                    "{category}/{name}: codes mismatch\n  expected {:?}\n  actual   {:?}",
                    expected_codes, actual_codes
                ));
            }
            if actual_exit != expected.exit_code {
                failures.push(format!(
                    "{category}/{name}: exit code mismatch (expected {}, actual {})",
                    expected.exit_code, actual_exit
                ));
            }
        }

        for (fixture, was_seen) in seen {
            if !was_seen {
                failures.push(format!(
                    "{category}/{fixture}: expectations entry has no matching fixture"
                ));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "security corpus expectations diverged:\n{}",
        failures.join("\n")
    );
}

/// Mirrors the CLI `diagnostics` pipeline so the test contract matches
/// what users actually invoke.
fn run_pipeline(path: &Path) -> (Vec<String>, i32) {
    let bytes = fs::read(path).expect("read fixture");
    let limits = ResourceLimits::default();

    if is_packaged_nodx(&bytes) {
        // Security corpus today is plain-text only; reject packaged input
        // here would mask a regression of category placement.
        panic!(
            "{}: security corpus does not yet cover packaged inputs",
            path.display()
        );
    }

    let diagnostics: Vec<Diagnostic> = match parse_bytes_with_limits(&bytes, limits) {
        Ok(doc) => Validator::default().validate(&doc),
        Err(diag) => vec![*diag],
    };

    let exit = exit_code_for(&diagnostics);

    let mut codes: Vec<String> = diagnostics.into_iter().map(|d| d.code).collect();
    codes.sort();
    codes.dedup();
    (codes, exit)
}

/// Mirrors `nodx-cli::exit_code_for`.
fn exit_code_for(diagnostics: &[Diagnostic]) -> i32 {
    let mut exit = 0i32;
    for diag in diagnostics {
        let level = match diag.severity.as_str() {
            "fatal" | "error" => {
                if diag.code == "NODX-E024" {
                    3
                } else {
                    2
                }
            }
            _ => 0,
        };
        if level > exit {
            exit = level;
        }
    }
    exit
}

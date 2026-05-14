//! CLI integration smoke tests.
//!
//! These tests pin the user-facing contract of the `nodx` binary: exit codes,
//! the JSON shape of `diagnostics --format json`, and the routing of each
//! subcommand. The conformance script exercises byte parity across parsers;
//! this file exercises the CLI surface itself, which the script never sees.
//!
//! Exit code contract (per `NODX-RFC-0001` §24):
//!   0  success
//!   1  I/O or CLI usage error
//!   2  parse / validation / security failure
//!   3  unsupported required profile (`NODX-E024`)

use std::path::PathBuf;
use std::process::{Command, Stdio};

fn nodx_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_nodx"))
}

fn workspace_root() -> PathBuf {
    let mut here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    here.pop();
    here.pop();
    here
}

fn run(args: &[&str]) -> (i32, String, String) {
    let output = Command::new(nodx_bin())
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn nodx");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn ast_on_minimal_emits_canonical_json() {
    let path = workspace_root().join("examples/minimal.nodx");
    let (code, stdout, stderr) = run(&["ast", path.to_str().unwrap()]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(
        stdout.contains("\"schema\":\"nodx/1.0\""),
        "stdout: {stdout}"
    );
}

#[test]
fn validate_on_minimal_is_clean() {
    let path = workspace_root().join("examples/minimal.nodx");
    let (code, stdout, _) = run(&["validate", path.to_str().unwrap(), "--format", "json"]);
    assert_eq!(code, 0);
    assert_eq!(stdout.trim(), "[]");
}

#[test]
fn diagnostics_json_shape_is_stable() {
    let path = workspace_root().join("spec/tests/negative/e006-duplicate-id.nodx");
    let (code, stdout, _) = run(&["diagnostics", path.to_str().unwrap(), "--format", "json"]);
    assert_eq!(code, 2, "duplicate-id is severity=error -> exit 2");
    assert!(
        stdout.contains("\"code\":\"NODX-E006\""),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("\"severity\""));
    assert!(stdout.contains("\"line\""));
    assert!(stdout.contains("\"column\""));
    assert!(stdout.contains("\"target\""));
}

#[test]
fn unsupported_required_profile_exits_with_3() {
    let path = workspace_root().join("spec/tests/negative/e024-unsupported-required-profile.nodx");
    let (code, _stdout, _stderr) = run(&["validate", path.to_str().unwrap(), "--format", "json"]);
    assert_eq!(
        code, 3,
        "NODX-E024 maps to CLI exit code 3 per RFC §24 and docs/reference/diagnostics.md"
    );
}

#[test]
fn unknown_subcommand_exits_with_1_and_prints_usage() {
    let (code, _stdout, stderr) = run(&["nonsense-command"]);
    assert_eq!(code, 1);
    // The CLI prints the full usage banner on unknown input; we assert on the
    // banner header rather than the word "unknown" (which the binary does not
    // emit) so the test is robust to wording tweaks.
    assert!(
        stderr.contains("nodx 1.0") && stderr.contains("Commands:"),
        "stderr should contain the usage banner; got: {stderr}"
    );
}

#[test]
fn missing_file_exits_with_io_error() {
    let (code, _stdout, _stderr) = run(&["ast", "/this/path/does/not/exist.nodx"]);
    assert_eq!(code, 1, "I/O errors should map to exit code 1");
}

#[test]
fn fatal_yaml_exits_with_2() {
    let path = workspace_root().join("spec/tests/security/yaml-hostile/alias.nodx");
    let (code, stdout, _) = run(&["diagnostics", path.to_str().unwrap(), "--format", "json"]);
    assert_eq!(code, 2);
    assert!(
        stdout.contains("\"code\":\"NODX-E019\""),
        "yaml-hostile alias should emit NODX-E019 in stdout: {stdout}"
    );
}

#[test]
fn ncp_and_semantic_subcommands_route_correctly() {
    let path = workspace_root().join("examples/minimal.nodx");
    let (code, stdout, _) = run(&["ncp", path.to_str().unwrap()]);
    assert_eq!(code, 0);
    assert!(stdout.contains("\"schema\":\"nodx-ncp/1.0\""));

    let (code, stdout, _) = run(&["semantic", path.to_str().unwrap()]);
    assert_eq!(code, 0);
    assert!(!stdout.is_empty());
}

#[test]
fn html_subcommand_emits_csp_meta() {
    let path = workspace_root().join("examples/minimal.nodx");
    let (code, stdout, _) = run(&["html", path.to_str().unwrap()]);
    assert_eq!(code, 0);
    assert!(
        stdout.contains("Content-Security-Policy"),
        "html should embed CSP"
    );
}

#[test]
fn package_inspect_routes_to_package_subcommand() {
    let path = workspace_root().join("examples/extended-showcase-bundled.nodx");
    let (code, stdout, _) = run(&["package", "inspect", path.to_str().unwrap()]);
    assert_eq!(code, 0);
    assert!(stdout.contains("entry"), "stdout: {stdout}");
}

#[test]
fn integrity_emits_base64url_digest() {
    let path = workspace_root().join("examples/minimal.nodx");
    let (code, stdout, _) = run(&["integrity", path.to_str().unwrap()]);
    assert_eq!(code, 0);
    let digest = stdout.trim();
    // base64url charset, no padding required in this output.
    assert!(
        digest
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
        "digest must be base64url: {digest}"
    );
    assert!(
        digest.len() >= 43,
        "sha-256 base64url is at least 43 chars: {digest}"
    );
}

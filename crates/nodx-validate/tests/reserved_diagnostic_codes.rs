//! Guard: `NODX-E011` and `NODX-E015` are reserved in baseline 1.0.
//!
//! The codes are part of the stable registry (see NODX-RFC-0001 §23.1 and
//! `docs/reference/diagnostics.md`) but no baseline processor emits them.
//! `E011` is held back until the `::include` profile lands; `E015` is held
//! back until the renderer lossy-fallback semantics are split from
//! `NODX-E026`.
//!
//! This test scans every Rust source file in the workspace and fails if a
//! non-test, non-fixture occurrence of either code is found. Touch this test
//! intentionally when activating the corresponding future profile.

use std::{fs, path::Path};

const RESERVED: &[&str] = &["NODX-E011", "NODX-E015"];

#[test]
fn reserved_codes_have_no_emitters() {
    let workspace_root = workspace_root();
    let crates_dir = workspace_root.join("crates");

    let mut hits: Vec<String> = Vec::new();
    walk_rs(&crates_dir, &mut |path, contents| {
        // Skip the guard test itself.
        if path.ends_with("reserved_diagnostic_codes.rs") {
            return;
        }
        for code in RESERVED {
            if contents.contains(code) {
                // Allow comment-only mentions (docs, RFC pointers).
                if mentions_only_in_comments(contents, code) {
                    continue;
                }
                hits.push(format!("{}: contains {}", path.display(), code));
            }
        }
    });

    assert!(
        hits.is_empty(),
        "reserved diagnostic codes were found as live emitters:\n{}\n\
         Update RFC §23.1 and docs/reference/diagnostics.md if you are \
         activating one of these codes intentionally.",
        hits.join("\n")
    );
}

fn workspace_root() -> std::path::PathBuf {
    let mut here = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
    // crates/nodx-validate -> repo root
    here.pop();
    here.pop();
    here
}

fn walk_rs(dir: &Path, visit: &mut dyn FnMut(&Path, &str)) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_rs(&path, visit);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs")
            && let Ok(contents) = fs::read_to_string(&path)
        {
            visit(&path, &contents);
        }
    }
}

fn mentions_only_in_comments(contents: &str, code: &str) -> bool {
    for line in contents.lines() {
        if !line.contains(code) {
            continue;
        }
        let trimmed = line.trim_start();
        let is_comment = trimmed.starts_with("//")
            || trimmed.starts_with("/*")
            || trimmed.starts_with("*")
            || trimmed.starts_with("#[doc");
        if !is_comment {
            return false;
        }
    }
    true
}

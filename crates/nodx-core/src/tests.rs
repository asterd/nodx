use super::*;

#[test]
fn parses_core_blocks() {
    let doc = parse_str("# Title {#t}\n\n:::note {type=\"warning\"}\nBody **x**.\n:::\n");
    assert_eq!(doc.body.len(), 2);
    assert_eq!(doc.body[0].node_type, "heading");
    assert_eq!(doc.body[1].node_type, "note");
    assert!(canonical_json(&doc).contains("\"type\":\"strong\""));
}

#[test]
fn parses_tables_and_lists() {
    let doc = parse_str("- [ ] Todo\n- [x] Done\n\n| A | B |\n| - | - |\n| 1 | 2 |\n");
    assert_eq!(
        doc.body[0].attrs.get("kind").map(String::as_str),
        Some("task")
    );
    assert_eq!(doc.body[1].node_type, "table");
}

#[test]
fn unclosed_delimited_block_emits_diagnostic() {
    let doc = parse_str("::::section\nbody\n:::note\nx\n");
    assert!(
        doc.diagnostics
            .iter()
            .any(|d| d.code == "NODX-E005" && d.message.contains("Unclosed"))
    );
}

#[test]
fn front_matter_block_sequence_of_mappings() {
    let doc =
        parse_str("---\nschema: nodx/0.1\nauthors:\n  - name: Alice\n  - name: Bob\n---\n\nBody\n");
    let authors = doc.meta.get("authors").expect("authors");
    match authors {
        Value::List(items) => {
            assert_eq!(items.len(), 2);
            if let Value::Map(m) = &items[0] {
                assert!(matches!(m.get("name"), Some(Value::String(s)) if s == "Alice"));
            } else {
                panic!("expected first author to be a map");
            }
        }
        other => panic!("expected list, got {:?}", other),
    }
}

#[test]
fn front_matter_preserves_unknown_nested_metadata() {
    let doc = parse_str(
        "---\nschema: nodx/0.1\nx-extra:\n  flag: true\n  values: [1, two]\n---\n\nBody\n",
    );
    let extra = doc.meta.get("x-extra").expect("x-extra");
    match extra {
        Value::Map(map) => {
            assert!(matches!(map.get("flag"), Some(Value::Bool(true))));
            assert!(matches!(map.get("values"), Some(Value::List(items)) if items.len() == 2));
        }
        other => panic!("expected map, got {other:?}"),
    }
}

#[test]
fn front_matter_numbers_are_canonicalized() {
    let doc = parse_str("---\nschema: nodx/1.0\nvars:\n  release: 1.0\n---\n\nBody\n");
    assert_eq!(canonical_json(&doc).matches("\"release\":1").count(), 1);
    assert!(!canonical_json(&doc).contains("\"release\":1.0"));
}

#[test]
fn front_matter_rejects_hostile_yaml_constructs() {
    for source in [
        "---\nschema: nodx/0.1\nbase: &base x\n---\n",
        "---\nschema: nodx/0.1\ncopy: *base\n---\n",
        "---\nschema: nodx/0.1\ntagged: !!str x\n---\n",
        "---\nschema: nodx/0.1\n<<: {title: x}\n---\n",
        "---\nschema: nodx/0.1\nschema: nodx/0.1\n---\n",
        "---\n[not, string]: x\n---\n",
        "---\nvalue: .nan\n---\n",
        "---\nvalue: 0x10\n---\n",
        "---\ndate: 2026-05-11\n---\n",
        "---\nschema: nodx/0.1\n...\n---\n",
    ] {
        let doc = parse_str(source);
        assert!(
            doc.diagnostics.iter().any(|d| d.code == "NODX-E019"),
            "missing NODX-E019 for {source:?}: {:?}",
            doc.diagnostics
        );
    }
}

#[test]
fn yaml_hostile_corpus_all_emit_e019() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir = root.join("spec/tests/security/yaml-hostile");
    let mut count = 0;
    for entry in std::fs::read_dir(&dir).expect("read yaml-hostile dir") {
        let entry = entry.expect("dirent");
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("nodx") {
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("read fixture");
        let doc = parse_str(&source);
        assert!(
            doc.diagnostics.iter().any(|d| d.code == "NODX-E019"),
            "missing NODX-E019 for {:?}: {:?}",
            path.file_name(),
            doc.diagnostics
        );
        count += 1;
    }
    assert!(count >= 30, "expected at least 30 hostile YAML fixtures, found {count}");
}

#[test]
fn front_matter_block_scalar_allows_literal_specials() {
    // `&`/`*`/`!` inside a literal block scalar are normal scalar bytes and must
    // not trigger NODX-E019.
    let doc = parse_str("---\nschema: nodx/1.0\nnotice: |\n  & anchor\n  * bullet text\n  !important call-out\n---\n\n# A\n");
    assert!(
        doc.diagnostics.iter().all(|d| d.code != "NODX-E019"),
        "got: {:?}",
        doc.diagnostics
    );
}

#[test]
fn labelled_close_produces_same_ast_as_plain() {
    let plain = parse_str(":::note\nBody.\n:::\n");
    let labelled = parse_str(":::note\nBody.\n::: note\n");
    assert_eq!(canonical_json(&plain), canonical_json(&labelled));
    assert!(labelled.diagnostics.iter().all(|d| d.code != "NODX-E005"));
}

#[test]
fn labelled_close_mismatch_emits_diagnostic_but_recovers() {
    let doc = parse_str("::::section\n:::note\nBody.\n::: figure\n::::\n");
    let codes: Vec<_> = doc.diagnostics.iter().map(|d| d.code.as_str()).collect();
    assert!(codes.contains(&"NODX-E005"));
    assert_eq!(doc.body.len(), 1);
    assert_eq!(doc.body[0].node_type, "section");
}

#[test]
fn style_block_text_is_preserved_verbatim() {
    // NODS audit/sanitization is the validator's responsibility; the parser only
    // captures the raw block contents.
    let doc = parse_str(":::style\nh1 { color: red; }\n:::\n");
    assert_eq!(doc.body[0].text.as_deref(), Some("h1 { color: red; }"));
}

#[test]
fn opener_is_not_confused_with_labelled_close() {
    let doc = parse_str(":::table\n:::row\n:::cell\nA\n:::\n:::\n:::\n");
    assert_eq!(doc.body.len(), 1);
    assert_eq!(doc.body[0].node_type, "table");
    assert_eq!(doc.body[0].children[0].node_type, "row");
    assert_eq!(doc.body[0].children[0].children[0].node_type, "cell");
}

#[test]
fn parse_rejects_zip_inputs() {
    // Packaged inputs must go through `nodx-package` first.
    let bytes = build_zip_with_duplicate_path();
    let err = parse_bytes(&bytes).expect_err("zip bytes should not reach the parser");
    assert_eq!(err.code, "NODX-E001");
}

#[test]
fn parser_leaves_semantic_validation_to_validator() {
    let doc = parse_str(
        "# A {#x}\n\n## B {#x}\n\n@[missing]\n\n:::image {src=\"../secret.png\"}\n:::\n\n| A | B |\n| - | - |\n| 1 |\n",
    );
    let codes: Vec<_> = doc.diagnostics.iter().map(|d| d.code.as_str()).collect();
    assert!(!codes.contains(&"NODX-E006"));
    assert!(!codes.contains(&"NODX-E007"));
    assert!(!codes.contains(&"NODX-E009"));
    assert!(!codes.contains(&"NODX-E025"));
}

#[test]
fn parses_mark_sub_and_sup() {
    let doc = parse_str("==mark== ~sub~ ^sup^\n");
    let json = canonical_json(&doc);
    assert!(json.contains("\"type\":\"mark\""));
    assert!(json.contains("\"type\":\"sub\""));
    assert!(json.contains("\"type\":\"sup\""));
}

#[test]
fn sha256_matches_known_vector() {
    assert_eq!(
        sha256_base64url(b"abc"),
        "sha256-ungWv48Bz-pBQUDeXa4iI7ADYaOWF3qctBD_YfIAFa0"
    );
}

fn build_zip_with_duplicate_path() -> Vec<u8> {
    let mut out = Vec::new();
    let mut entries: Vec<(String, Vec<u8>, u32)> = Vec::new();
    for (name, data) in [
        ("mimetype", b"application/nodx+zip".to_vec()),
        (
            "manifest.yaml",
            b"schema: nodx-package/0.1\nentry: doc.nodx\n".to_vec(),
        ),
        ("doc.nodx", b"# A".to_vec()),
        ("doc.nodx", b"# B".to_vec()),
    ] {
        let local_offset = out.len() as u32;
        out.extend_from_slice(&[0x50, 0x4b, 0x03, 0x04]);
        out.extend_from_slice(&[20, 0]);
        out.extend_from_slice(&[0, 0]);
        out.extend_from_slice(&[0, 0]);
        out.extend_from_slice(&[0, 0, 0, 0]);
        out.extend_from_slice(&[0, 0, 0, 0]);
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(&[0, 0]);
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(&data);
        entries.push((name.to_string(), data, local_offset));
    }
    let cd_offset = out.len() as u32;
    for (name, data, local_offset) in &entries {
        out.extend_from_slice(&[0x50, 0x4b, 0x01, 0x02]);
        out.extend_from_slice(&[20, 0]);
        out.extend_from_slice(&[20, 0]);
        out.extend_from_slice(&[0, 0]);
        out.extend_from_slice(&[0, 0]);
        out.extend_from_slice(&[0, 0, 0, 0]);
        out.extend_from_slice(&[0, 0, 0, 0]);
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(&[0, 0]);
        out.extend_from_slice(&[0, 0]);
        out.extend_from_slice(&[0, 0]);
        out.extend_from_slice(&[0, 0]);
        out.extend_from_slice(&[0, 0, 0, 0]);
        out.extend_from_slice(&local_offset.to_le_bytes());
        out.extend_from_slice(name.as_bytes());
    }
    let cd_size = (out.len() as u32) - cd_offset;
    out.extend_from_slice(&[0x50, 0x4b, 0x05, 0x06]);
    out.extend_from_slice(&[0, 0]);
    out.extend_from_slice(&[0, 0]);
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&cd_size.to_le_bytes());
    out.extend_from_slice(&cd_offset.to_le_bytes());
    out.extend_from_slice(&[0, 0]);
    out
}

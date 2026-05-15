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
fn parses_lite_blocks_heading_ids_vars_and_link_attrs() {
    let doc = parse_str(
        "# Title #intro\n\n::note {type=\"info\"}\nHello {{reviewer}} and [guide](docs/guide.nodx){title=\"Open guide\" rel=\"help\"}.\n::note\n",
    );
    assert_eq!(doc.body[0].id.as_deref(), Some("intro"));
    assert_eq!(doc.body[1].node_type, "note");
    let json = canonical_json(&doc);
    assert!(json.contains("\"namespace\":\"vars\""));
    assert!(json.contains("\"title\":\"Open guide\""));
    assert!(json.contains("\"rel\":\"help\""));
}

#[test]
fn escaped_heading_light_id_stays_literal_text() {
    let doc = parse_str("# Title \\#intro\n");
    assert_eq!(doc.body[0].id, None);
    assert!(canonical_json(&doc).contains("Title #intro"));
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
fn pipe_tables_preserve_alignment_and_cell_attrs() {
    let doc =
        parse_str("| Name | Amount |\n| :--- | ---: |\n| {colspan=2 align=\"center\"} Total | |\n");
    let table = &doc.body[0];
    let header = &table.children[0].children;
    assert_eq!(
        header[0].attrs.get("align").map(String::as_str),
        Some("left")
    );
    assert_eq!(
        header[1].attrs.get("align").map(String::as_str),
        Some("right")
    );
    let first_body_cell = &table.children[1].children[0];
    assert_eq!(
        first_body_cell.attrs.get("colspan").map(String::as_str),
        Some("2")
    );
    assert_eq!(
        first_body_cell.attrs.get("align").map(String::as_str),
        Some("center")
    );
    assert_eq!(plain_inlines(&first_body_cell.inlines), "Total");
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
fn thematic_break_recognises_three_markers() {
    // Three patterns, each on an isolated line. The opening `---` is not the
    // first line of the document so it cannot be confused with front matter.
    let doc = parse_str("Above paragraph.\n\n---\n\nMiddle.\n\n***\n\nMore.\n\n___\n\nTail.\n");
    let types: Vec<&str> = doc.body.iter().map(|n| n.node_type.as_str()).collect();
    assert_eq!(
        types,
        vec![
            "paragraph",
            "hr",
            "paragraph",
            "hr",
            "paragraph",
            "hr",
            "paragraph",
        ]
    );
    // The `hr` node is a leaf: empty children/inlines/text. The byte-stable
    // canonical JSON must serialise it as such.
    for node in &doc.body {
        if node.node_type == "hr" {
            assert!(node.children.is_empty());
            assert!(node.inlines.is_empty());
            assert!(node.text.is_none());
            assert!(node.classes.is_empty());
            assert!(node.attrs.is_empty());
            assert!(node.id.is_none());
        }
    }
    assert!(canonical_json(&doc).contains("\"type\":\"hr\""));
}

#[test]
fn thematic_break_with_internal_whitespace_is_paragraph() {
    // CommonMark accepts `- - -`; NODX intentionally does not. The line stays
    // a paragraph so the byte-stable AST is unambiguous.
    let doc = parse_str("Before.\n\n- - -\n\nAfter.\n");
    assert!(doc.body.iter().all(|n| n.node_type != "hr"));
}

#[test]
fn thematic_break_too_few_markers_is_paragraph() {
    let doc = parse_str("Before.\n\n--\n\nAfter.\n");
    assert!(doc.body.iter().all(|n| n.node_type != "hr"));
}

#[test]
fn thematic_break_inside_front_matter_does_not_apply() {
    // First `---` opens front matter, second closes it. Neither is an `hr`.
    let doc = parse_str("---\ntitle: Demo\n---\n\nBody.\n\n---\n\nTail.\n");
    // After front matter, the standalone `---` between paragraphs is an `hr`.
    assert_eq!(doc.body.len(), 3);
    assert_eq!(doc.body[0].node_type, "paragraph");
    assert_eq!(doc.body[1].node_type, "hr");
    assert_eq!(doc.body[2].node_type, "paragraph");
}

#[test]
fn thematic_break_first_line_when_no_front_matter() {
    // A document that begins with `---` followed by a non-`---` line on line 2
    // opens a front matter and fails to close it. To get an HR at the start
    // we'd need a blank line first, mimicking the structural rule.
    let doc = parse_str("\n---\n\nBody.\n");
    assert_eq!(doc.body[0].node_type, "hr");
}

#[test]
fn underscore_emphasis_works_at_word_boundaries() {
    // RFC §12 (PR2): underscore emphasis follows the CommonMark intraword
    // rule. `_em_` and `__strong__` produce nested inlines, the same as
    // `*em*` / `**strong**`.
    let doc = parse_str("# T\n\nHere is _emphasis_ and __strong__ next to *star em* text.\n");
    let inlines = &doc.body[1].inlines;
    let kinds: Vec<&str> = inlines
        .iter()
        .map(|inl| match inl {
            Inline::Em(_) => "em",
            Inline::Strong(_) => "strong",
            Inline::Text(_) => "text",
            _ => "other",
        })
        .collect();
    assert!(kinds.contains(&"em"));
    assert!(kinds.contains(&"strong"));
    let json = canonical_json(&doc);
    assert!(json.contains("\"type\":\"em\""));
    assert!(json.contains("\"type\":\"strong\""));
}

#[test]
fn intraword_underscores_stay_literal() {
    // `snake_case`, `__init__,` (with no left ws? actually `as __init__,` will
    // open emphasis under CommonMark rules). The reliable literal cases are
    // alnum-flanked: `snake_case` and `snake__case`.
    let doc = parse_str("Identifiers like snake_case and snake__case stay literal.\n");
    let json = canonical_json(&doc);
    assert!(!json.contains("\"type\":\"em\""));
    assert!(!json.contains("\"type\":\"strong\""));
}

#[test]
fn code_span_with_multiple_backticks() {
    // CommonMark code span rule: the closing run must match the opening run
    // length exactly. Trimming a single surrounding space is applied when
    // both sides have one and the content is not all-spaces.
    let doc = parse_str("Show `a` then ``two ` ticks`` and ```three `` runs``` here.\n");
    let codes: Vec<&str> = doc.body[0]
        .inlines
        .iter()
        .filter_map(|inl| match inl {
            Inline::Code(text) => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(codes, vec!["a", "two ` ticks", "three `` runs"]);
}

#[test]
fn extended_backslash_escapes() {
    let doc = parse_str("Escaped: \\_ \\! \\. \\- \\+ \\< \\> \\\\ \\\" \\' done.\n");
    let text = plain_inlines(&doc.body[0].inlines);
    assert!(text.contains("_"));
    assert!(text.contains("!"));
    assert!(text.contains("."));
    assert!(text.contains("\\"));
    assert!(text.contains("\""));
    assert!(text.contains("'"));
    // No emphasis was produced because the underscore was escaped.
    assert!(!canonical_json(&doc).contains("\"type\":\"em\""));
}

#[test]
fn hard_line_break_inside_paragraph() {
    let doc = parse_str("First half\\\nsecond half.\n");
    let inlines = &doc.body[0].inlines;
    assert!(matches!(inlines[1], Inline::LineBreak));
    assert!(canonical_json(&doc).contains("\"type\":\"line-break\""));
}

#[test]
fn extended_list_markers_normalize_to_kind() {
    // Each marker variant produces the same canonical `kind` value: `- `,
    // `* `, `+ ` → unordered; `1.`, `1)` → ordered. The literal marker is
    // *not* preserved.
    let doc = parse_str("- a\n- b\n\n* c\n* d\n\n+ e\n+ f\n\n1. g\n2. h\n\n1) i\n2) j\n");
    let lists: Vec<&str> = doc
        .body
        .iter()
        .filter(|n| n.node_type == "list")
        .map(|n| n.attrs.get("kind").map(String::as_str).unwrap_or(""))
        .collect();
    assert_eq!(
        lists,
        vec!["unordered", "unordered", "unordered", "ordered", "ordered"]
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
    assert!(
        count >= 30,
        "expected at least 30 hostile YAML fixtures, found {count}"
    );
}

#[test]
fn front_matter_block_scalar_allows_literal_specials() {
    // `&`/`*`/`!` inside a literal block scalar are normal scalar bytes and must
    // not trigger NODX-E019.
    let doc = parse_str(
        "---\nschema: nodx/1.0\nnotice: |\n  & anchor\n  * bullet text\n  !important call-out\n---\n\n# A\n",
    );
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
fn parses_mark_strike_sub_and_sup() {
    let doc = parse_str("==mark=={bg=\"#ffe08a\"} ~~strike~~ ~sub~ ^sup^\n");
    let json = canonical_json(&doc);
    assert!(json.contains("\"type\":\"mark\""));
    assert!(json.contains("\"background-color\":\"#ffe08a\""));
    assert!(json.contains("\"type\":\"strike\""));
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

#[test]
fn nodes_per_document_limit_emits_e012_and_stops_parsing() {
    let limits = ResourceLimits {
        nodes_per_document: 5,
        ..ResourceLimits::default()
    };
    let source = "# A\n\n# B\n\n# C\n\n# D\n\n# E\n\n# F\n\n# G\n";
    let doc = parse_str_with_limits(source, limits);
    assert!(
        doc.diagnostics
            .iter()
            .any(|d| d.code == "NODX-E012" && d.message.contains("Node count")),
        "missing node count diagnostic: {:?}",
        doc.diagnostics
    );
    assert!(
        doc.body.len() <= 5,
        "parser must stop creating nodes after limit: got {}",
        doc.body.len()
    );
}

#[test]
fn block_nesting_depth_limit_emits_e012() {
    let limits = ResourceLimits {
        block_nesting_depth: 2,
        ..ResourceLimits::default()
    };
    let source = ":::a\n:::b\n:::c\nbody\n:::\n:::\n:::\n";
    let doc = parse_str_with_limits(source, limits);
    assert!(
        doc.diagnostics
            .iter()
            .any(|d| d.code == "NODX-E012" && d.message.contains("nesting depth")),
        "missing nesting depth diagnostic: {:?}",
        doc.diagnostics
    );
}

#[test]
fn limits_do_not_fire_under_default_caps_on_small_docs() {
    let doc = parse_str("# A\n\nP1\n\nP2\n");
    assert!(doc.diagnostics.iter().all(|d| d.code != "NODX-E012"));
}

#[test]
fn string_parser_source_limit_fails_before_body_parse() {
    let limits = ResourceLimits {
        source_bytes: 4,
        ..ResourceLimits::default()
    };
    let doc = parse_str_with_limits("# A\n\nbody\n", limits);
    assert!(doc.body.is_empty());
    assert!(
        doc.diagnostics
            .iter()
            .any(|d| d.code == "NODX-E012" && d.severity == "fatal")
    );
}

#[test]
fn string_parser_line_limit_fails_before_body_parse() {
    let limits = ResourceLimits {
        line_length: 3,
        ..ResourceLimits::default()
    };
    let doc = parse_str_with_limits("abcd\n# B\n", limits);
    assert!(doc.body.is_empty());
    assert!(
        doc.diagnostics
            .iter()
            .any(|d| d.code == "NODX-E012" && d.message.contains("Line length"))
    );
}

#[test]
fn configured_attribute_value_limit_is_enforced() {
    let limits = ResourceLimits {
        attribute_value_bytes: 3,
        ..ResourceLimits::default()
    };
    let doc = parse_str_with_limits(":::note {title=\"abcd\" label=\"ok\"}\n:::\n", limits);
    assert_eq!(doc.body[0].attrs.get("title"), None);
    assert_eq!(doc.body[0].attrs.get("label"), Some(&"ok".to_string()));
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

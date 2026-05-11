use nodx_core::{ResourceLimits, crc32};
use nodx_package::{Package, PackageDiagnostic};

fn build_zip(entries: Vec<(&str, Vec<u8>, u32, u16)>) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central: Vec<(String, u32, u32, u32, u32, u16)> = Vec::new();
    for (name, data, mode, compression) in entries {
        let local_offset = out.len() as u32;
        let crc = crc32(&data);
        out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        out.extend_from_slice(&20u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&compression.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(&data);
        central.push((name.to_string(), data.len() as u32, crc, local_offset, mode, compression));
    }
    let cd_offset = out.len() as u32;
    for (name, len, crc, local_offset, mode, compression) in &central {
        out.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        out.extend_from_slice(&20u16.to_le_bytes());
        out.extend_from_slice(&20u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&compression.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&((*mode) << 16).to_le_bytes());
        out.extend_from_slice(&local_offset.to_le_bytes());
        out.extend_from_slice(name.as_bytes());
    }
    let cd_size = out.len() as u32 - cd_offset;
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&(central.len() as u16).to_le_bytes());
    out.extend_from_slice(&(central.len() as u16).to_le_bytes());
    out.extend_from_slice(&cd_size.to_le_bytes());
    out.extend_from_slice(&cd_offset.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

fn minimal_manifest(entry: &str) -> Vec<u8> {
    format!("schema: nodx-package/1.0\nentry: {entry}\n").into_bytes()
}

fn open(zip: &[u8]) -> Result<Package, PackageDiagnostic> {
    Package::open(zip, ResourceLimits::default())
}

#[test]
fn corpus_rejects_absolute_path() {
    let zip = build_zip(vec![
        ("mimetype", b"application/nodx+zip".to_vec(), 0o100644, 0),
        ("manifest.yaml", minimal_manifest("/abs/doc.nodx"), 0o100644, 0),
        ("/abs/doc.nodx", b"# A\n".to_vec(), 0o100644, 0),
    ]);
    let err = open(&zip).unwrap_err();
    assert_eq!(err.code, "NODX-E010");
}

#[test]
fn corpus_rejects_traversal_in_entry() {
    let zip = build_zip(vec![
        ("mimetype", b"application/nodx+zip".to_vec(), 0o100644, 0),
        ("manifest.yaml", minimal_manifest("../doc.nodx"), 0o100644, 0),
        ("../doc.nodx", b"# A\n".to_vec(), 0o100644, 0),
    ]);
    assert!(open(&zip).is_err());
}

#[test]
fn corpus_rejects_backslash_path() {
    let zip = build_zip(vec![
        ("mimetype", b"application/nodx+zip".to_vec(), 0o100644, 0),
        ("manifest.yaml", minimal_manifest("a\\b.nodx"), 0o100644, 0),
        ("a\\b.nodx", b"# A\n".to_vec(), 0o100644, 0),
    ]);
    assert!(open(&zip).is_err());
}

#[test]
fn corpus_rejects_control_char_path() {
    let zip = build_zip(vec![
        ("mimetype", b"application/nodx+zip".to_vec(), 0o100644, 0),
        ("manifest.yaml", minimal_manifest("a\x07b.nodx"), 0o100644, 0),
        ("a\x07b.nodx", b"# A\n".to_vec(), 0o100644, 0),
    ]);
    assert!(open(&zip).is_err());
}

#[test]
fn corpus_rejects_dot_segment() {
    let zip = build_zip(vec![
        ("mimetype", b"application/nodx+zip".to_vec(), 0o100644, 0),
        ("manifest.yaml", minimal_manifest("./doc.nodx"), 0o100644, 0),
        ("./doc.nodx", b"# A\n".to_vec(), 0o100644, 0),
    ]);
    assert!(open(&zip).is_err());
}

#[test]
fn corpus_rejects_empty_segment() {
    let zip = build_zip(vec![
        ("mimetype", b"application/nodx+zip".to_vec(), 0o100644, 0),
        ("manifest.yaml", minimal_manifest("a//b.nodx"), 0o100644, 0),
        ("a//b.nodx", b"# A\n".to_vec(), 0o100644, 0),
    ]);
    assert!(open(&zip).is_err());
}

#[test]
fn corpus_rejects_duplicate_entry() {
    let zip = build_zip(vec![
        ("mimetype", b"application/nodx+zip".to_vec(), 0o100644, 0),
        ("manifest.yaml", minimal_manifest("doc.nodx"), 0o100644, 0),
        ("doc.nodx", b"# A\n".to_vec(), 0o100644, 0),
        ("doc.nodx", b"# B\n".to_vec(), 0o100644, 0),
    ]);
    assert!(open(&zip).is_err());
}

#[test]
fn corpus_rejects_symlink_mode() {
    let zip = build_zip(vec![
        ("mimetype", b"application/nodx+zip".to_vec(), 0o100644, 0),
        ("manifest.yaml", minimal_manifest("doc.nodx"), 0o100644, 0),
        ("doc.nodx", b"# A\n".to_vec(), 0o120777, 0),
    ]);
    assert!(open(&zip).is_err());
}

#[test]
fn corpus_rejects_directory_mode() {
    let zip = build_zip(vec![
        ("mimetype", b"application/nodx+zip".to_vec(), 0o100644, 0),
        ("manifest.yaml", minimal_manifest("doc.nodx"), 0o100644, 0),
        ("doc.nodx", b"# A\n".to_vec(), 0o040755, 0),
    ]);
    assert!(open(&zip).is_err());
}

#[test]
fn corpus_rejects_nested_zip() {
    let zip = build_zip(vec![
        ("mimetype", b"application/nodx+zip".to_vec(), 0o100644, 0),
        ("manifest.yaml", minimal_manifest("doc.nodx"), 0o100644, 0),
        ("doc.nodx", b"# A\n".to_vec(), 0o100644, 0),
        ("inner.zip", b"PK\x03\x04demo".to_vec(), 0o100644, 0),
    ]);
    let err = open(&zip).unwrap_err();
    assert_eq!(err.code, "NODX-E010");
}

#[test]
fn corpus_rejects_unknown_compression() {
    let zip = build_zip(vec![
        ("mimetype", b"application/nodx+zip".to_vec(), 0o100644, 0),
        ("manifest.yaml", minimal_manifest("doc.nodx"), 0o100644, 0),
        ("doc.nodx", b"# A\n".to_vec(), 0o100644, 99),
    ]);
    assert!(open(&zip).is_err());
}

#[test]
fn corpus_rejects_mimetype_compressed() {
    let zip = build_zip(vec![
        ("mimetype", b"application/nodx+zip".to_vec(), 0o100644, 8),
        ("manifest.yaml", minimal_manifest("doc.nodx"), 0o100644, 0),
        ("doc.nodx", b"# A\n".to_vec(), 0o100644, 0),
    ]);
    assert!(open(&zip).is_err());
}

#[test]
fn corpus_rejects_missing_mimetype() {
    let zip = build_zip(vec![
        ("manifest.yaml", minimal_manifest("doc.nodx"), 0o100644, 0),
        ("mimetype", b"application/nodx+zip".to_vec(), 0o100644, 0),
        ("doc.nodx", b"# A\n".to_vec(), 0o100644, 0),
    ]);
    let err = open(&zip).unwrap_err();
    assert!(err.message.contains("First ZIP entry"));
}

#[test]
fn corpus_rejects_wrong_mimetype_bytes() {
    let zip = build_zip(vec![
        ("mimetype", b"application/zip".to_vec(), 0o100644, 0),
        ("manifest.yaml", minimal_manifest("doc.nodx"), 0o100644, 0),
        ("doc.nodx", b"# A\n".to_vec(), 0o100644, 0),
    ]);
    assert!(open(&zip).is_err());
}

#[test]
fn corpus_rejects_corrupt_eocd() {
    let mut zip = build_zip(vec![
        ("mimetype", b"application/nodx+zip".to_vec(), 0o100644, 0),
        ("manifest.yaml", minimal_manifest("doc.nodx"), 0o100644, 0),
        ("doc.nodx", b"# A\n".to_vec(), 0o100644, 0),
    ]);
    let n = zip.len();
    zip[n - 22] = 0;
    assert!(open(&zip).is_err());
}

#[test]
fn corpus_rejects_manifest_missing_entry() {
    let zip = build_zip(vec![
        ("mimetype", b"application/nodx+zip".to_vec(), 0o100644, 0),
        ("manifest.yaml", b"schema: nodx-package/1.0\n".to_vec(), 0o100644, 0),
        ("doc.nodx", b"# A\n".to_vec(), 0o100644, 0),
    ]);
    assert!(open(&zip).is_err());
}

#[test]
fn corpus_rejects_manifest_entry_not_in_zip() {
    let zip = build_zip(vec![
        ("mimetype", b"application/nodx+zip".to_vec(), 0o100644, 0),
        ("manifest.yaml", minimal_manifest("missing.nodx"), 0o100644, 0),
        ("doc.nodx", b"# A\n".to_vec(), 0o100644, 0),
    ]);
    assert!(open(&zip).is_err());
}

#[test]
fn corpus_rejects_digest_mismatch() {
    let manifest = b"schema: nodx-package/1.0\nentry: doc.nodx\nentries:\n  - path: doc.nodx\n    size: 4\n    sha256: sha256-FAKE\n".to_vec();
    let zip = build_zip(vec![
        ("mimetype", b"application/nodx+zip".to_vec(), 0o100644, 0),
        ("manifest.yaml", manifest, 0o100644, 0),
        ("doc.nodx", b"# A\n".to_vec(), 0o100644, 0),
    ]);
    let err = open(&zip).unwrap_err();
    assert_eq!(err.code, "NODX-E021");
}

#[test]
fn corpus_rejects_manifest_size_mismatch() {
    let manifest = b"schema: nodx-package/1.0\nentry: doc.nodx\nentries:\n  - path: doc.nodx\n    size: 99\n".to_vec();
    let zip = build_zip(vec![
        ("mimetype", b"application/nodx+zip".to_vec(), 0o100644, 0),
        ("manifest.yaml", manifest, 0o100644, 0),
        ("doc.nodx", b"# A\n".to_vec(), 0o100644, 0),
    ]);
    let err = open(&zip).unwrap_err();
    assert_eq!(err.code, "NODX-E021");
}

#[test]
fn corpus_rejects_manifest_anchor() {
    let manifest = b"schema: &x nodx-package/1.0\nentry: doc.nodx\n".to_vec();
    let zip = build_zip(vec![
        ("mimetype", b"application/nodx+zip".to_vec(), 0o100644, 0),
        ("manifest.yaml", manifest, 0o100644, 0),
        ("doc.nodx", b"# A\n".to_vec(), 0o100644, 0),
    ]);
    assert_eq!(open(&zip).unwrap_err().code, "NODX-E019");
}

#[test]
fn corpus_rejects_manifest_merge_key() {
    let manifest = b"schema: nodx-package/1.0\nentry: doc.nodx\n<<: foo\n".to_vec();
    let zip = build_zip(vec![
        ("mimetype", b"application/nodx+zip".to_vec(), 0o100644, 0),
        ("manifest.yaml", manifest, 0o100644, 0),
        ("doc.nodx", b"# A\n".to_vec(), 0o100644, 0),
    ]);
    assert_eq!(open(&zip).unwrap_err().code, "NODX-E019");
}

#[test]
fn corpus_rejects_manifest_duplicate_key() {
    let manifest = b"schema: nodx-package/1.0\nschema: nodx-package/1.0\nentry: doc.nodx\n".to_vec();
    let zip = build_zip(vec![
        ("mimetype", b"application/nodx+zip".to_vec(), 0o100644, 0),
        ("manifest.yaml", manifest, 0o100644, 0),
        ("doc.nodx", b"# A\n".to_vec(), 0o100644, 0),
    ]);
    assert_eq!(open(&zip).unwrap_err().code, "NODX-E019");
}

#[test]
fn corpus_rejects_manifest_unknown_field() {
    let manifest = b"schema: nodx-package/1.0\nentry: doc.nodx\nbogus: 1\n".to_vec();
    let zip = build_zip(vec![
        ("mimetype", b"application/nodx+zip".to_vec(), 0o100644, 0),
        ("manifest.yaml", manifest, 0o100644, 0),
        ("doc.nodx", b"# A\n".to_vec(), 0o100644, 0),
    ]);
    assert_eq!(open(&zip).unwrap_err().code, "NODX-E019");
}

#[test]
fn corpus_rejects_manifest_flow_style() {
    let manifest = b"schema: nodx-package/1.0\nentry: doc.nodx\nprofiles: [core]\n".to_vec();
    let zip = build_zip(vec![
        ("mimetype", b"application/nodx+zip".to_vec(), 0o100644, 0),
        ("manifest.yaml", manifest, 0o100644, 0),
        ("doc.nodx", b"# A\n".to_vec(), 0o100644, 0),
    ]);
    assert_eq!(open(&zip).unwrap_err().code, "NODX-E019");
}

#[test]
fn corpus_rejects_wrong_schema() {
    let manifest = b"schema: nodx-package/0.1\nentry: doc.nodx\n".to_vec();
    let zip = build_zip(vec![
        ("mimetype", b"application/nodx+zip".to_vec(), 0o100644, 0),
        ("manifest.yaml", manifest, 0o100644, 0),
        ("doc.nodx", b"# A\n".to_vec(), 0o100644, 0),
    ]);
    assert!(open(&zip).is_err());
}

#[test]
fn corpus_rejects_missing_schema() {
    let manifest = b"entry: doc.nodx\n".to_vec();
    let zip = build_zip(vec![
        ("mimetype", b"application/nodx+zip".to_vec(), 0o100644, 0),
        ("manifest.yaml", manifest, 0o100644, 0),
        ("doc.nodx", b"# A\n".to_vec(), 0o100644, 0),
    ]);
    assert!(open(&zip).is_err());
}

#[test]
fn corpus_rejects_path_byte_limit() {
    let long = "a".repeat(513);
    let path = format!("{long}.nodx");
    let manifest = format!("schema: nodx-package/1.0\nentry: {path}\n").into_bytes();
    let zip = build_zip(vec![
        ("mimetype", b"application/nodx+zip".to_vec(), 0o100644, 0),
        ("manifest.yaml", manifest, 0o100644, 0),
        (&path, b"# A\n".to_vec(), 0o100644, 0),
    ]);
    assert!(open(&zip).is_err());
}

#[test]
fn corpus_rejects_path_segment_limit() {
    let path = (0..10).map(|i| format!("a{i}")).collect::<Vec<_>>().join("/");
    let full = format!("{path}/doc.nodx");
    let manifest = format!("schema: nodx-package/1.0\nentry: {full}\n").into_bytes();
    let zip = build_zip(vec![
        ("mimetype", b"application/nodx+zip".to_vec(), 0o100644, 0),
        ("manifest.yaml", manifest, 0o100644, 0),
        (&full, b"# A\n".to_vec(), 0o100644, 0),
    ]);
    assert!(open(&zip).is_err());
}

#[test]
fn corpus_rejects_signature_missing_when_declared() {
    let manifest = b"schema: nodx-package/1.0\nentry: doc.nodx\nsignature: signatures/missing.jws\n".to_vec();
    let zip = build_zip(vec![
        ("mimetype", b"application/nodx+zip".to_vec(), 0o100644, 0),
        ("manifest.yaml", manifest, 0o100644, 0),
        ("doc.nodx", b"# A\n".to_vec(), 0o100644, 0),
    ]);
    assert!(open(&zip).is_err());
}

#[test]
fn corpus_rejects_zero_size_compressed_with_uncompressed() {
    // A central-directory entry that claims compressed_size=0 but uncompressed_size>0
    // is a ZIP-bomb signal.
    let mut zip = build_zip(vec![
        ("mimetype", b"application/nodx+zip".to_vec(), 0o100644, 0),
        ("manifest.yaml", minimal_manifest("doc.nodx"), 0o100644, 0),
        ("doc.nodx", b"# A\n".to_vec(), 0o100644, 0),
    ]);
    // find central directory for doc.nodx and zero compressed_size
    let cd_signature = [0x50u8, 0x4b, 0x01, 0x02];
    let mut i = 0;
    let mut patched = false;
    while i + 4 <= zip.len() {
        if zip[i..i + 4] == cd_signature {
            // compressed_size at offset 20, uncompressed_size at 24
            // patch only the doc.nodx entry by checking name later
            let name_len = u16::from_le_bytes([zip[i + 28], zip[i + 29]]) as usize;
            let name_start = i + 46;
            let name = std::str::from_utf8(&zip[name_start..name_start + name_len]).unwrap();
            if name == "doc.nodx" {
                zip[i + 20] = 0;
                zip[i + 21] = 0;
                zip[i + 22] = 0;
                zip[i + 23] = 0;
                patched = true;
                break;
            }
        }
        i += 1;
    }
    assert!(patched);
    assert!(open(&zip).is_err());
}

#[test]
fn corpus_rejects_path_traversal_via_percent_encoding() {
    let zip = build_zip(vec![
        ("mimetype", b"application/nodx+zip".to_vec(), 0o100644, 0),
        ("manifest.yaml", minimal_manifest("a/%2e%2e/b.nodx"), 0o100644, 0),
        ("a/%2e%2e/b.nodx", b"# A\n".to_vec(), 0o100644, 0),
    ]);
    assert!(open(&zip).is_err());
}

#[test]
fn corpus_accepts_minimal_valid_package() {
    let zip = build_zip(vec![
        ("mimetype", b"application/nodx+zip".to_vec(), 0o100644, 0),
        ("manifest.yaml", minimal_manifest("doc.nodx"), 0o100644, 0),
        ("doc.nodx", b"# A\n".to_vec(), 0o100644, 0),
    ]);
    let pkg = open(&zip).expect("valid package");
    assert_eq!(pkg.entry_path(), "doc.nodx");
}

#[test]
fn corpus_accepts_signature_declared_and_present() {
    let manifest = b"schema: nodx-package/1.0\nentry: doc.nodx\nsignature: signatures/document.jws\n".to_vec();
    let zip = build_zip(vec![
        ("mimetype", b"application/nodx+zip".to_vec(), 0o100644, 0),
        ("manifest.yaml", manifest, 0o100644, 0),
        ("doc.nodx", b"# A\n".to_vec(), 0o100644, 0),
        ("signatures/document.jws", b"detached".to_vec(), 0o100644, 0),
    ]);
    let pkg = open(&zip).expect("valid package with signature");
    assert_eq!(pkg.signature_path(), Some("signatures/document.jws"));
}

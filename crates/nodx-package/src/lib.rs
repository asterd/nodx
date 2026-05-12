#![forbid(unsafe_code)]

mod inflate;
mod manifest;

use std::collections::{BTreeMap, BTreeSet};

use nodx_core::{
    Document, ResourceLimits, Value, crc32 as core_crc32, parse_str,
    sha256_base64url as core_sha256_base64url,
};
use nodx_url::{ResourcePolicy, UrlError};

use crate::manifest::parse_package_manifest;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageDiagnostic {
    pub code: String,
    pub severity: String,
    pub message: String,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Package {
    entry: String,
    signature_path: Option<String>,
    profiles_required: Vec<String>,
    profiles_optional: Vec<String>,
    component_paths: Vec<String>,
    theme_paths: Vec<String>,
    fs: PackageFs,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PackageFs {
    files: BTreeMap<String, Vec<u8>>,
}

impl Package {
    pub fn open(input: &[u8], limits: ResourceLimits) -> Result<Self, PackageDiagnostic> {
        let entries = zip_entries(input, limits)?;
        if entries.len() > limits.package_file_count {
            return Err(diag_code("NODX-E012", "Package file count limit exceeded."));
        }
        let total_size = entries.iter().try_fold(0usize, |acc, entry| {
            checked_add(acc, entry.uncompressed_size)
        })?;
        if total_size > limits.package_uncompressed_bytes {
            return Err(diag_code(
                "NODX-E012",
                "Package uncompressed size limit exceeded.",
            ));
        }

        let first = entries
            .iter()
            .min_by_key(|entry| entry.local_offset)
            .ok_or_else(|| diag("Package is empty."))?;
        if first.name != "mimetype" {
            return Err(diag("First ZIP entry must be mimetype."));
        }
        if first.compression != 0 {
            return Err(diag("mimetype entry must be stored uncompressed."));
        }

        let mut fs = PackageFs::default();
        for entry in &entries {
            let data = zip_read_entry(input, entry, limits)?;
            if looks_like_zip(&data) {
                return Err(diag_code(
                    "NODX-E010",
                    "Nested ZIP archives are not supported.",
                ));
            }
            fs.files.insert(entry.name.clone(), data);
        }

        if fs.read("mimetype") != Some(&b"application/nodx+zip"[..]) {
            return Err(diag("Invalid NODX package mimetype."));
        }
        let manifest = fs
            .read("manifest.yaml")
            .ok_or_else(|| diag("Package is missing manifest.yaml."))?;
        let manifest_text =
            std::str::from_utf8(manifest).map_err(|_| diag("Package manifest is not UTF-8."))?;
        let manifest_data = parse_package_manifest(manifest_text)?;
        if !matches!(manifest_data.schema.as_deref(), Some("nodx-package/1.0")) {
            return Err(diag("Package manifest schema must be `nodx-package/1.0`."));
        }
        if manifest_data.entries.len() > limits.manifest_entries {
            return Err(diag_code(
                "NODX-E012",
                "Package manifest entry limit exceeded.",
            ));
        }
        for manifest_entry in &manifest_data.entries {
            validate_package_path(&manifest_entry.path, limits)?;
            let Some(data) = fs.read(&manifest_entry.path) else {
                return Err(diag_code(
                    "NODX-E021",
                    "Manifest lists a missing package entry.",
                ));
            };
            if let Some(size) = manifest_entry.size
                && size != data.len()
            {
                return Err(diag_code(
                    "NODX-E021",
                    "Package manifest size does not match entry bytes.",
                ));
            }
            if let Some(expected) = &manifest_entry.sha256
                && expected != &sha256_base64url(data)
            {
                return Err(diag_code("NODX-E021", "Package digest mismatch."));
            }
        }
        for path in manifest_data
            .components
            .iter()
            .chain(manifest_data.themes.iter())
        {
            validate_package_path(path, limits)?;
            if fs.read(path).is_none() {
                return Err(diag_code(
                    "NODX-E021",
                    "Manifest lists a missing package extension.",
                ));
            }
            if !manifest_data
                .entries
                .iter()
                .any(|entry| entry.path == *path)
            {
                return Err(diag_code(
                    "NODX-E021",
                    "Package extension paths must also be listed in manifest entries.",
                ));
            }
        }
        let entry = manifest_data
            .entry
            .clone()
            .ok_or_else(|| diag("Package manifest is missing entry."))?;
        validate_package_path(&entry, limits)?;
        if fs.read(&entry).is_none() {
            return Err(diag("Package entry document is missing."));
        }
        let signature_path = match &manifest_data.signature {
            Some(path) => {
                validate_package_path(path, limits)?;
                if fs.read(path).is_none() {
                    return Err(diag("Package manifest references a missing signature."));
                }
                Some(path.clone())
            }
            None => None,
        };
        Ok(Self {
            entry,
            signature_path,
            profiles_required: manifest_data.profiles_required,
            profiles_optional: manifest_data.profiles_optional,
            component_paths: manifest_data.components,
            theme_paths: manifest_data.themes,
            fs,
        })
    }

    pub fn entry_path(&self) -> &str {
        &self.entry
    }

    pub fn entry_bytes(&self) -> &[u8] {
        self.fs
            .read(&self.entry)
            .expect("entry exists after package verification")
    }

    pub fn signature_path(&self) -> Option<&str> {
        self.signature_path.as_deref()
    }

    pub fn profiles_required(&self) -> &[String] {
        &self.profiles_required
    }

    pub fn profiles_optional(&self) -> &[String] {
        &self.profiles_optional
    }

    pub fn component_paths(&self) -> &[String] {
        &self.component_paths
    }

    pub fn theme_paths(&self) -> &[String] {
        &self.theme_paths
    }

    pub fn fs(&self) -> &PackageFs {
        &self.fs
    }
}

pub fn apply_package_extensions(
    doc: &Document,
    package: &Package,
) -> Result<Document, PackageDiagnostic> {
    let mut out = doc.clone();
    let mut components = match out.meta.remove("components") {
        Some(Value::List(items)) => items,
        _ => Vec::new(),
    };
    for path in package.component_paths() {
        components.push(Value::Map(component_definition(package, path)?));
    }
    if !components.is_empty() {
        out.meta
            .insert("components".to_string(), Value::List(components));
    }

    let mut stylesheets = match out.meta.remove("stylesheets") {
        Some(Value::List(items)) => items,
        _ => Vec::new(),
    };
    for path in package.theme_paths() {
        let text = package_text(package, path)?;
        stylesheets.push(Value::String(text));
    }
    if !stylesheets.is_empty() {
        out.meta
            .insert("stylesheets".to_string(), Value::List(stylesheets));
    }
    Ok(out)
}

fn component_definition(
    package: &Package,
    path: &str,
) -> Result<BTreeMap<String, Value>, PackageDiagnostic> {
    let text = package_text(package, path)?;
    let parsed = parse_str(&text);
    let mut map = BTreeMap::new();
    let name = match parsed.meta.get("name") {
        Some(Value::String(name)) => name.clone(),
        _ => path
            .rsplit('/')
            .next()
            .unwrap_or(path)
            .trim_end_matches(".nodx")
            .to_string(),
    };
    map.insert("name".to_string(), Value::String(name));
    map.insert(
        "template".to_string(),
        Value::String(strip_front_matter(&text)),
    );
    if let Some(Value::String(style)) = parsed.meta.get("style")
        && !style.trim().is_empty()
    {
        map.insert("style".to_string(), Value::String(style.clone()));
    }
    Ok(map)
}

fn package_text(package: &Package, path: &str) -> Result<String, PackageDiagnostic> {
    let bytes = package
        .fs()
        .read(path)
        .ok_or_else(|| diag_code("NODX-E021", "Package extension path is missing."))?;
    std::str::from_utf8(bytes)
        .map(str::to_string)
        .map_err(|_| diag_code("NODX-E002", "Package extension is not UTF-8."))
}

fn strip_front_matter(text: &str) -> String {
    if !text.starts_with("---\n") {
        return text.to_string();
    }
    let Some(end) = text[4..].find("\n---") else {
        return text.to_string();
    };
    let mut body_start = 4 + end + 4;
    if text.as_bytes().get(body_start) == Some(&b'\r') {
        body_start += 1;
    }
    if text.as_bytes().get(body_start) == Some(&b'\n') {
        body_start += 1;
    }
    text[body_start..].to_string()
}

impl PackageFs {
    pub fn read(&self, path: &str) -> Option<&[u8]> {
        self.files.get(path).map(Vec::as_slice)
    }

    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.files.keys().map(String::as_str)
    }
}

pub fn read_packaged_nodx_entry(
    input: &[u8],
    limits: ResourceLimits,
) -> Result<Vec<u8>, PackageDiagnostic> {
    Ok(Package::open(input, limits)?.entry_bytes().to_vec())
}

pub fn sha256_base64url(input: &[u8]) -> String {
    core_sha256_base64url(input)
}

#[derive(Clone, Debug)]
struct ZipEntry {
    name: String,
    compression: u16,
    crc32: u32,
    compressed_size: usize,
    uncompressed_size: usize,
    local_offset: usize,
    external_attrs: u32,
}

fn zip_entries(input: &[u8], limits: ResourceLimits) -> Result<Vec<ZipEntry>, PackageDiagnostic> {
    let eocd = find_eocd(input).ok_or_else(|| diag("ZIP end of central directory not found."))?;
    if read_u16(input, eocd + 4)? != 0 || read_u16(input, eocd + 6)? != 0 {
        return Err(diag("Multi-disk ZIP packages are not supported."));
    }
    let disk_count = read_u16(input, eocd + 8)?;
    let count = read_u16(input, eocd + 10)?;
    let cd_size = read_u32(input, eocd + 12)?;
    let cd_offset = read_u32(input, eocd + 16)?;
    if disk_count == 0xffff || count == 0xffff || cd_size == 0xffff_ffff || cd_offset == 0xffff_ffff
    {
        return Err(diag("ZIP64 packages are not supported."));
    }
    if disk_count != count {
        return Err(diag("Invalid ZIP central directory."));
    }

    let mut pos = cd_offset as usize;
    let mut entries = Vec::new();
    let mut seen_names = BTreeSet::new();
    for _ in 0..count {
        if read_u32(input, pos)? != 0x0201_4b50 {
            return Err(diag("Invalid ZIP central directory."));
        }
        let flags = read_u16(input, pos + 8)?;
        let compression = read_u16(input, pos + 10)?;
        let crc32 = read_u32(input, pos + 16)?;
        let compressed_size_u32 = read_u32(input, pos + 20)?;
        let uncompressed_size_u32 = read_u32(input, pos + 24)?;
        let local_offset_u32 = read_u32(input, pos + 42)?;
        if compressed_size_u32 == 0xffff_ffff
            || uncompressed_size_u32 == 0xffff_ffff
            || local_offset_u32 == 0xffff_ffff
        {
            return Err(diag("ZIP64 packages are not supported."));
        }
        let compressed_size = compressed_size_u32 as usize;
        let uncompressed_size = uncompressed_size_u32 as usize;
        let local_offset = local_offset_u32 as usize;
        if uncompressed_size > limits.package_entry_bytes {
            return Err(diag_code("NODX-E012", "Package entry size limit exceeded."));
        }
        if compressed_size == 0 && uncompressed_size > 0 {
            return Err(diag_code(
                "NODX-E012",
                "Package compression ratio limit exceeded.",
            ));
        }
        if compressed_size > 0
            && uncompressed_size > compressed_size.saturating_mul(limits.package_compression_ratio)
        {
            return Err(diag_code(
                "NODX-E012",
                "Package compression ratio limit exceeded.",
            ));
        }
        let name_len = read_u16(input, pos + 28)? as usize;
        let extra_len = read_u16(input, pos + 30)? as usize;
        let comment_len = read_u16(input, pos + 32)? as usize;
        let external_attrs = read_u32(input, pos + 38)?;
        let name_start = pos + 46;
        let name_end = checked_add(name_start, name_len)?;
        let extra_end = checked_add(name_end, extra_len)?;
        if extra_end > input.len() {
            return Err(diag("ZIP entry name is out of bounds."));
        }
        reject_zip64_extra(&input[name_end..extra_end])?;
        let raw_name = std::str::from_utf8(&input[name_start..name_end])
            .map_err(|_| diag("ZIP entry name is not UTF-8."))?;
        let name = validate_package_path(raw_name, limits)?;
        reject_special_file(external_attrs)?;
        if flags & 1 != 0 {
            return Err(diag("Encrypted ZIP entries are not supported."));
        }
        if flags & 8 != 0 {
            return Err(diag("ZIP data descriptors are not supported."));
        }
        if compression != 0 && compression != 8 {
            return Err(diag(
                "Unsupported ZIP compression method (only stored and deflate are supported).",
            ));
        }
        if !seen_names.insert(name.clone()) {
            return Err(diag("Duplicate package entry path."));
        }
        entries.push(ZipEntry {
            name,
            compression,
            crc32,
            compressed_size,
            uncompressed_size,
            local_offset,
            external_attrs,
        });
        pos = checked_add(extra_end, comment_len)?;
    }
    Ok(entries)
}

fn zip_read_entry(
    input: &[u8],
    entry: &ZipEntry,
    limits: ResourceLimits,
) -> Result<Vec<u8>, PackageDiagnostic> {
    let pos = entry.local_offset;
    if read_u32(input, pos)? != 0x0403_4b50 {
        return Err(diag("Invalid ZIP local header."));
    }
    let flags = read_u16(input, pos + 6)?;
    let compression = read_u16(input, pos + 8)?;
    let crc32 = read_u32(input, pos + 14)?;
    let compressed_size = read_u32(input, pos + 18)? as usize;
    let uncompressed_size = read_u32(input, pos + 22)? as usize;
    if flags & 1 != 0 {
        return Err(diag("Encrypted ZIP entries are not supported."));
    }
    if flags & 8 != 0 {
        return Err(diag("ZIP data descriptors are not supported."));
    }
    if compression != entry.compression
        || crc32 != entry.crc32
        || compressed_size != entry.compressed_size
        || uncompressed_size != entry.uncompressed_size
    {
        return Err(diag("ZIP local header does not match central directory."));
    }
    let name_len = read_u16(input, pos + 26)? as usize;
    let extra_len = read_u16(input, pos + 28)? as usize;
    let name_start = pos + 30;
    let name_end = checked_add(name_start, name_len)?;
    let extra_end = checked_add(name_end, extra_len)?;
    if extra_end > input.len() {
        return Err(diag("ZIP entry data is out of bounds."));
    }
    reject_zip64_extra(&input[name_end..extra_end])?;
    let raw_name = std::str::from_utf8(&input[name_start..name_end])
        .map_err(|_| diag("ZIP local header name is not UTF-8."))?;
    let name = validate_package_path(raw_name, limits)?;
    if name != entry.name {
        return Err(diag(
            "ZIP local header name does not match central directory.",
        ));
    }
    reject_special_file(entry.external_attrs)?;
    let data_start = extra_end;
    let data_end = checked_add(data_start, entry.compressed_size)?;
    if data_end > input.len() {
        return Err(diag("ZIP entry data is out of bounds."));
    }
    let compressed = &input[data_start..data_end];
    let data = if entry.compression == 0 {
        if entry.compressed_size != entry.uncompressed_size {
            return Err(diag("Stored ZIP entry has inconsistent sizes."));
        }
        compressed.to_vec()
    } else {
        inflate::inflate(compressed, entry.uncompressed_size)
            .map_err(|msg| diag(&format!("DEFLATE error: {msg}")))?
    };
    if data.len() != entry.uncompressed_size {
        return Err(diag("ZIP DEFLATE produced an unexpected length."));
    }
    if core_crc32(&data) != entry.crc32 {
        return Err(diag("ZIP CRC mismatch."));
    }
    Ok(data)
}

fn reject_zip64_extra(extra: &[u8]) -> Result<(), PackageDiagnostic> {
    let mut pos = 0usize;
    while pos + 4 <= extra.len() {
        let header = u16::from_le_bytes([extra[pos], extra[pos + 1]]);
        let len = u16::from_le_bytes([extra[pos + 2], extra[pos + 3]]) as usize;
        pos += 4;
        let end = checked_add(pos, len)?;
        if end > extra.len() {
            return Err(diag("Invalid ZIP extra field."));
        }
        if header == 0x0001 {
            return Err(diag("ZIP64 packages are not supported."));
        }
        pos = end;
    }
    if pos != extra.len() {
        return Err(diag("Invalid ZIP extra field."));
    }
    Ok(())
}

fn reject_special_file(external_attrs: u32) -> Result<(), PackageDiagnostic> {
    let mode = external_attrs >> 16;
    if mode == 0 {
        return Ok(());
    }
    let file_type = mode & 0o170000;
    if file_type == 0o100000 {
        return Ok(());
    }
    Err(diag("ZIP special files are not supported."))
}

fn find_eocd(input: &[u8]) -> Option<usize> {
    let min = 22;
    if input.len() < min {
        return None;
    }
    let start = input.len().saturating_sub(65_557);
    (start..=input.len() - min)
        .rev()
        .find(|&pos| input.get(pos..pos + 4) == Some(b"PK\x05\x06"))
}

pub(crate) fn validate_package_path(
    path: &str,
    limits: ResourceLimits,
) -> Result<String, PackageDiagnostic> {
    if path.len() > limits.package_path_bytes {
        return Err(diag_code("NODX-E012", "Package path byte limit exceeded."));
    }
    if path.split('/').count() > limits.package_path_segments {
        return Err(diag_code(
            "NODX-E012",
            "Package path segment limit exceeded.",
        ));
    }
    ResourcePolicy::new(limits)
        .normalize_package_path(path)
        .map_err(|err| match err {
            UrlError::PathTraversal | UrlError::AbsolutePath | UrlError::Backslash => {
                diag_code("NODX-E010", "Unsafe package path.")
            }
            _ => diag_code("NODX-E010", "Invalid package path."),
        })
}

fn looks_like_zip(data: &[u8]) -> bool {
    matches!(
        data.get(0..4),
        Some(b"PK\x03\x04") | Some(b"PK\x05\x06") | Some(b"PK\x07\x08")
    )
}

fn read_u16(input: &[u8], pos: usize) -> Result<u16, PackageDiagnostic> {
    let bytes = input
        .get(pos..pos + 2)
        .ok_or_else(|| diag("Unexpected end of ZIP data."))?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn read_u32(input: &[u8], pos: usize) -> Result<u32, PackageDiagnostic> {
    let bytes = input
        .get(pos..pos + 4)
        .ok_or_else(|| diag("Unexpected end of ZIP data."))?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn checked_add(a: usize, b: usize) -> Result<usize, PackageDiagnostic> {
    a.checked_add(b).ok_or_else(|| diag("ZIP offset overflow."))
}

pub(crate) fn diag(message: &str) -> PackageDiagnostic {
    diag_code("NODX-E012", message)
}

pub(crate) fn diag_code(code: &str, message: &str) -> PackageDiagnostic {
    PackageDiagnostic {
        code: code.to_string(),
        severity: severity_for(code).to_string(),
        message: message.to_string(),
    }
}

fn severity_for(code: &str) -> &'static str {
    match code {
        "NODX-E010" | "NODX-E021" => "error",
        "NODX-E019" => "fatal",
        _ => "fatal",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_valid_package_as_read_only_fs() {
        let package = Package::open(&package_bytes(false), ResourceLimits::default()).unwrap();
        assert_eq!(package.entry_path(), "doc.nodx");
        assert_eq!(package.fs().read("doc.nodx"), Some(&b"# A\n"[..]));
        assert!(package.fs().paths().any(|path| path == "manifest.yaml"));
    }

    #[test]
    fn digest_mismatch_is_e021() {
        let err = Package::open(&package_bytes(true), ResourceLimits::default()).unwrap_err();
        assert_eq!(err.code, "NODX-E021");
        assert_eq!(err.severity, "error");
    }

    #[test]
    fn crc_mismatch_is_rejected() {
        let mut bytes = package_bytes(false);
        let doc = b"# A\n";
        let pos = bytes
            .windows(doc.len())
            .position(|window| window == doc)
            .expect("doc bytes");
        bytes[pos] = b'!';
        let err = Package::open(&bytes, ResourceLimits::default()).unwrap_err();
        assert!(err.message.contains("CRC"));
    }

    #[test]
    fn duplicate_normalized_names_are_rejected() {
        let bytes = build_zip(vec![
            ("mimetype", b"application/nodx+zip".to_vec(), 0o100644, 0),
            (
                "manifest.yaml",
                b"schema: nodx-package/1.0\nentry: a/b.nodx\n".to_vec(),
                0o100644,
                0,
            ),
            ("a/b.nodx", b"# A\n".to_vec(), 0o100644, 0),
            (" a/b.nodx ", b"# B\n".to_vec(), 0o100644, 0),
        ]);
        let err = Package::open(&bytes, ResourceLimits::default()).unwrap_err();
        assert!(err.message.contains("Duplicate") || err.message.contains("path"));
    }

    #[test]
    fn symlink_entries_are_rejected() {
        let bytes = build_zip(vec![
            ("mimetype", b"application/nodx+zip".to_vec(), 0o100644, 0),
            (
                "manifest.yaml",
                b"schema: nodx-package/1.0\nentry: doc.nodx\n".to_vec(),
                0o100644,
                0,
            ),
            ("doc.nodx", b"# A\n".to_vec(), 0o120777, 0),
        ]);
        let err = Package::open(&bytes, ResourceLimits::default()).unwrap_err();
        assert!(err.message.contains("special"));
    }

    #[test]
    fn nested_zip_entries_are_rejected() {
        let bytes = build_zip(vec![
            ("mimetype", b"application/nodx+zip".to_vec(), 0o100644, 0),
            (
                "manifest.yaml",
                b"schema: nodx-package/1.0\nentry: doc.nodx\n".to_vec(),
                0o100644,
                0,
            ),
            ("doc.nodx", b"# A\n".to_vec(), 0o100644, 0),
            ("assets/nested.zip", b"PK\x03\x04demo".to_vec(), 0o100644, 0),
        ]);
        let err = Package::open(&bytes, ResourceLimits::default()).unwrap_err();
        assert_eq!(err.code, "NODX-E010");
    }

    #[test]
    fn hostile_manifest_anchor_is_rejected() {
        let bytes = build_zip(vec![
            ("mimetype", b"application/nodx+zip".to_vec(), 0o100644, 0),
            (
                "manifest.yaml",
                b"schema: &x nodx-package/1.0\nentry: doc.nodx\n".to_vec(),
                0o100644,
                0,
            ),
            ("doc.nodx", b"# A\n".to_vec(), 0o100644, 0),
        ]);
        let err = Package::open(&bytes, ResourceLimits::default()).unwrap_err();
        assert_eq!(err.code, "NODX-E019");
    }

    pub(crate) fn package_bytes(tamper_digest: bool) -> Vec<u8> {
        let doc = b"# A\n".to_vec();
        let digest = if tamper_digest {
            "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string()
        } else {
            sha256_base64url(&doc)
        };
        let manifest = format!(
            "schema: nodx-package/1.0\nentry: doc.nodx\nentries:\n  - path: doc.nodx\n    size: {}\n    sha256: {}\n",
            doc.len(),
            digest
        )
        .into_bytes();
        build_zip(vec![
            ("mimetype", b"application/nodx+zip".to_vec(), 0o100644, 0),
            ("manifest.yaml", manifest, 0o100644, 0),
            ("doc.nodx", doc, 0o100644, 0),
        ])
    }

    pub(crate) fn build_zip(entries: Vec<(&str, Vec<u8>, u32, u16)>) -> Vec<u8> {
        let mut out = Vec::new();
        let mut central = Vec::new();
        for (name, data, mode, compression) in entries {
            let local_offset = out.len() as u32;
            let crc = core_crc32(&data);
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
            central.push((
                name.to_string(),
                data.len() as u32,
                crc,
                local_offset,
                mode,
                compression,
            ));
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
}

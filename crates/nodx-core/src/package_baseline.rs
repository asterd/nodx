use std::collections::BTreeSet;

use crate::bytes::sha256_base64url;
use crate::diagnostic::Diagnostic;
use crate::front_matter::unquote;
use crate::limits::ResourceLimits;

pub(crate) fn read_packaged_nodx_entry(input: &[u8]) -> Result<Vec<u8>, Diagnostic> {
    let limits = ResourceLimits::default();
    let entries = zip_entries(input)?;
    if entries.len() > limits.package_file_count {
        return Err(package_diag_code(
            "NODX-E012",
            "Package file count limit exceeded.",
        ));
    }
    let total_size = entries.iter().try_fold(0usize, |acc, entry| {
        checked_add(acc, entry.uncompressed_size)
    })?;
    if total_size > limits.package_uncompressed_bytes {
        return Err(package_diag_code(
            "NODX-E012",
            "Package uncompressed size limit exceeded.",
        ));
    }
    let first = entries
        .iter()
        .min_by_key(|entry| entry.local_offset)
        .ok_or_else(|| package_diag("Package is empty."))?;
    if first.name != "mimetype" {
        return Err(package_diag("First ZIP entry must be mimetype."));
    }
    let mimetype = zip_read_stored(input, first)?;
    if mimetype != b"application/nodx+zip" {
        return Err(package_diag("Invalid NODX package mimetype."));
    }

    let manifest = entries
        .iter()
        .find(|entry| entry.name == "manifest.yaml")
        .ok_or_else(|| package_diag("Package is missing manifest.yaml."))?;
    let manifest_text = String::from_utf8(zip_read_stored(input, manifest)?)
        .map_err(|_| package_diag("Package manifest is not UTF-8."))?;
    let manifest_data = parse_package_manifest(&manifest_text);
    if manifest_data.schema.as_deref() != Some("nodx-package/0.1") {
        return Err(package_diag("Package manifest has an invalid schema."));
    }
    verify_manifest_entries(input, &entries, &manifest_data.entries)?;
    let entry_path = manifest_data
        .entry
        .ok_or_else(|| package_diag("Package manifest is missing entry."))?;
    let doc_entry = entries
        .iter()
        .find(|entry| entry.name == entry_path)
        .ok_or_else(|| package_diag("Package entry document is missing."))?;
    zip_read_stored(input, doc_entry)
}

struct ZipEntry {
    name: String,
    compression: u16,
    compressed_size: usize,
    uncompressed_size: usize,
    local_offset: usize,
}

fn zip_entries(input: &[u8]) -> Result<Vec<ZipEntry>, Diagnostic> {
    let eocd =
        find_eocd(input).ok_or_else(|| package_diag("ZIP end of central directory not found."))?;
    let count = read_u16(input, eocd + 10)? as usize;
    let cd_offset = read_u32(input, eocd + 16)? as usize;
    let mut pos = cd_offset;
    let mut entries = Vec::new();
    let mut seen_names = BTreeSet::new();
    for _ in 0..count {
        if read_u32(input, pos)? != 0x0201_4b50 {
            return Err(package_diag("Invalid ZIP central directory."));
        }
        let flags = read_u16(input, pos + 8)?;
        let compression = read_u16(input, pos + 10)?;
        let compressed_size = read_u32(input, pos + 20)? as usize;
        let uncompressed_size = read_u32(input, pos + 24)? as usize;
        let name_len = read_u16(input, pos + 28)? as usize;
        let extra_len = read_u16(input, pos + 30)? as usize;
        let comment_len = read_u16(input, pos + 32)? as usize;
        let local_offset = read_u32(input, pos + 42)? as usize;
        let name_start = pos + 46;
        let name_end = checked_add(name_start, name_len)?;
        if name_end > input.len() {
            return Err(package_diag("ZIP entry name is out of bounds."));
        }
        let name = std::str::from_utf8(&input[name_start..name_end])
            .map_err(|_| package_diag("ZIP entry name is not UTF-8."))?
            .to_string();
        validate_package_path(&name)?;
        if flags & 1 != 0 {
            return Err(package_diag("Encrypted ZIP entries are not supported."));
        }
        if compression != 0 {
            return Err(package_diag(
                "This minimal reference reader supports only stored ZIP entries.",
            ));
        }
        if !seen_names.insert(name.clone()) {
            return Err(package_diag("Duplicate package entry path."));
        }
        entries.push(ZipEntry {
            name,
            compression,
            compressed_size,
            uncompressed_size,
            local_offset,
        });
        pos = checked_add(name_end, checked_add(extra_len, comment_len)?)?;
    }
    Ok(entries)
}

fn zip_read_stored(input: &[u8], entry: &ZipEntry) -> Result<Vec<u8>, Diagnostic> {
    if entry.compression != 0 {
        return Err(package_diag(
            "This minimal reference reader supports only stored ZIP entries.",
        ));
    }
    if entry.compressed_size != entry.uncompressed_size {
        return Err(package_diag("Stored ZIP entry has inconsistent sizes."));
    }
    let pos = entry.local_offset;
    if read_u32(input, pos)? != 0x0403_4b50 {
        return Err(package_diag("Invalid ZIP local header."));
    }
    let name_len = read_u16(input, pos + 26)? as usize;
    let extra_len = read_u16(input, pos + 28)? as usize;
    let data_start = checked_add(pos + 30, checked_add(name_len, extra_len)?)?;
    let data_end = checked_add(data_start, entry.uncompressed_size)?;
    if data_end > input.len() {
        return Err(package_diag("ZIP entry data is out of bounds."));
    }
    Ok(input[data_start..data_end].to_vec())
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

struct PackageManifest {
    schema: Option<String>,
    entry: Option<String>,
    entries: Vec<PackageManifestEntry>,
}

struct PackageManifestEntry {
    path: String,
    size: Option<usize>,
    sha256: Option<String>,
}

fn parse_package_manifest(manifest: &str) -> PackageManifest {
    let mut out = PackageManifest {
        schema: None,
        entry: None,
        entries: Vec::new(),
    };
    let mut current: Option<PackageManifestEntry> = None;
    for line in manifest.lines() {
        let trimmed = line.trim();
        if let Some(value) = trimmed.strip_prefix("schema:") {
            out.schema = Some(unquote(value.trim()));
        } else if let Some(value) = trimmed.strip_prefix("entry:") {
            out.entry = Some(unquote(value.trim()));
        } else if let Some(value) = trimmed.strip_prefix("- path:") {
            if let Some(entry) = current.take() {
                out.entries.push(entry);
            }
            current = Some(PackageManifestEntry {
                path: unquote(value.trim()),
                size: None,
                sha256: None,
            });
        } else if let Some(value) = trimmed.strip_prefix("size:") {
            if let Some(entry) = current.as_mut() {
                entry.size = value.trim().parse::<usize>().ok();
            }
        } else if let Some(value) = trimmed.strip_prefix("sha256:") {
            if let Some(entry) = current.as_mut() {
                entry.sha256 = Some(unquote(value.trim()));
            }
        }
    }
    if let Some(entry) = current {
        out.entries.push(entry);
    }
    out
}

fn verify_manifest_entries(
    input: &[u8],
    entries: &[ZipEntry],
    manifest_entries: &[PackageManifestEntry],
) -> Result<(), Diagnostic> {
    for manifest_entry in manifest_entries {
        validate_package_path(&manifest_entry.path)?;
        let Some(zip_entry) = entries
            .iter()
            .find(|entry| entry.name == manifest_entry.path)
        else {
            return Err(package_diag("Manifest lists a missing package entry."));
        };
        let data = zip_read_stored(input, zip_entry)?;
        if let Some(size) = manifest_entry.size {
            if size != data.len() {
                return Err(package_diag_code(
                    "NODX-E021",
                    "Package manifest size does not match entry bytes.",
                ));
            }
        }
        if let Some(expected) = &manifest_entry.sha256 {
            if expected != &sha256_base64url(&data) {
                return Err(package_diag_code("NODX-E021", "Package digest mismatch."));
            }
        }
    }
    Ok(())
}

fn validate_package_path(path: &str) -> Result<(), Diagnostic> {
    if path.is_empty()
        || path.starts_with('/')
        || path.contains('\\')
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        Err(package_diag_code("NODX-E010", "Unsafe package path."))
    } else {
        if path.len() > 512 || path.split('/').count() > 8 {
            return Err(package_diag_code(
                "NODX-E012",
                "Package path limit exceeded.",
            ));
        }
        Ok(())
    }
}

fn read_u16(input: &[u8], pos: usize) -> Result<u16, Diagnostic> {
    let bytes = input
        .get(pos..pos + 2)
        .ok_or_else(|| package_diag("Unexpected end of ZIP data."))?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn read_u32(input: &[u8], pos: usize) -> Result<u32, Diagnostic> {
    let bytes = input
        .get(pos..pos + 4)
        .ok_or_else(|| package_diag("Unexpected end of ZIP data."))?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn checked_add(a: usize, b: usize) -> Result<usize, Diagnostic> {
    a.checked_add(b)
        .ok_or_else(|| package_diag("ZIP offset overflow."))
}

fn package_diag(message: &str) -> Diagnostic {
    package_diag_code("NODX-E012", message)
}

fn package_diag_code(code: &str, message: &str) -> Diagnostic {
    Diagnostic {
        code: code.to_string(),
        severity: "fatal".to_string(),
        message: message.to_string(),
        line: None,
        column: None,
        target: None,
    }
}

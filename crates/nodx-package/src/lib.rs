#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use nodx_url::{ResourceLimits, ResourcePolicy};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageDiagnostic {
    pub code: String,
    pub severity: String,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Package {
    entry: String,
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

        let mut fs = PackageFs::default();
        for entry in &entries {
            let data = zip_read_stored(input, entry, limits)?;
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
        let manifest_data = parse_package_manifest(manifest_text);
        if !matches!(
            manifest_data.schema.as_deref(),
            Some("nodx-package/1.0") | Some("nodx-package/0.1")
        ) {
            return Err(diag("Package manifest has an invalid schema."));
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
                return Err(diag("Manifest lists a missing package entry."));
            };
            if let Some(size) = manifest_entry.size {
                if size != data.len() {
                    return Err(diag_code(
                        "NODX-E021",
                        "Package manifest size does not match entry bytes.",
                    ));
                }
            }
            if let Some(expected) = &manifest_entry.sha256 {
                if expected != &sha256_base64url(data) {
                    return Err(diag_code("NODX-E021", "Package digest mismatch."));
                }
            }
        }
        let entry = manifest_data
            .entry
            .ok_or_else(|| diag("Package manifest is missing entry."))?;
        validate_package_path(&entry, limits)?;
        if fs.read(&entry).is_none() {
            return Err(diag("Package entry document is missing."));
        }
        Ok(Self { entry, fs })
    }

    pub fn entry_path(&self) -> &str {
        &self.entry
    }

    pub fn entry_bytes(&self) -> &[u8] {
        self.fs
            .read(&self.entry)
            .expect("entry exists after package verification")
    }

    pub fn fs(&self) -> &PackageFs {
        &self.fs
    }
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
        if compression != 0 {
            return Err(diag("Unsupported ZIP compression method."));
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

fn zip_read_stored(
    input: &[u8],
    entry: &ZipEntry,
    limits: ResourceLimits,
) -> Result<Vec<u8>, PackageDiagnostic> {
    if entry.compression != 0 {
        return Err(diag("Unsupported ZIP compression method."));
    }
    if entry.compressed_size != entry.uncompressed_size {
        return Err(diag("Stored ZIP entry has inconsistent sizes."));
    }
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
    let data_end = checked_add(data_start, entry.uncompressed_size)?;
    if data_end > input.len() {
        return Err(diag("ZIP entry data is out of bounds."));
    }
    let data = input[data_start..data_end].to_vec();
    if crc32_bytes(&data) != entry.crc32 {
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

#[derive(Clone, Debug, Default)]
struct PackageManifest {
    schema: Option<String>,
    entry: Option<String>,
    entries: Vec<PackageManifestEntry>,
}

#[derive(Clone, Debug)]
struct PackageManifestEntry {
    path: String,
    size: Option<usize>,
    sha256: Option<String>,
}

fn parse_package_manifest(manifest: &str) -> PackageManifest {
    let mut out = PackageManifest::default();
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

fn validate_package_path(path: &str, limits: ResourceLimits) -> Result<String, PackageDiagnostic> {
    if path.len() > 512 || path.split('/').count() > 8 {
        return Err(diag_code("NODX-E012", "Package path limit exceeded."));
    }
    ResourcePolicy::new(limits)
        .normalize_package_path(path)
        .map_err(|_| diag_code("NODX-E010", "Unsafe package path."))
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

fn unquote(raw: &str) -> String {
    if (raw.starts_with('"') && raw.ends_with('"'))
        || (raw.starts_with('\'') && raw.ends_with('\''))
    {
        raw[1..raw.len() - 1].to_string()
    } else {
        raw.to_string()
    }
}

fn diag(message: &str) -> PackageDiagnostic {
    diag_code("NODX-E012", message)
}

fn diag_code(code: &str, message: &str) -> PackageDiagnostic {
    PackageDiagnostic {
        code: code.to_string(),
        severity: if code == "NODX-E021" {
            "error".to_string()
        } else {
            "fatal".to_string()
        },
        message: message.to_string(),
    }
}

pub fn sha256_base64url(input: &[u8]) -> String {
    let mut out = String::from("sha256-");
    base64url_no_pad(&sha256(input), &mut out);
    out
}

fn crc32_bytes(input: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for &byte in input {
        crc ^= byte as u32;
        for _ in 0..8 {
            let mask = 0u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}

fn sha256(input: &[u8]) -> [u8; 32] {
    const H0: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let bit_len = (input.len() as u64) * 8;
    let mut msg = input.to_vec();
    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());
    let mut h = H0;
    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (i, word) in w.iter_mut().take(16).enumerate() {
            let start = i * 4;
            *word = u32::from_be_bytes([
                chunk[start],
                chunk[start + 1],
                chunk[start + 2],
                chunk[start + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }
    let mut out = [0u8; 32];
    for (i, word) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

fn base64url_no_pad(input: &[u8], out: &mut String) {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut i = 0;
    while i + 3 <= input.len() {
        let n = ((input[i] as u32) << 16) | ((input[i + 1] as u32) << 8) | input[i + 2] as u32;
        out.push(ALPHABET[((n >> 18) & 63) as usize] as char);
        out.push(ALPHABET[((n >> 12) & 63) as usize] as char);
        out.push(ALPHABET[((n >> 6) & 63) as usize] as char);
        out.push(ALPHABET[(n & 63) as usize] as char);
        i += 3;
    }
    match input.len() - i {
        1 => {
            let n = (input[i] as u32) << 16;
            out.push(ALPHABET[((n >> 18) & 63) as usize] as char);
            out.push(ALPHABET[((n >> 12) & 63) as usize] as char);
        }
        2 => {
            let n = ((input[i] as u32) << 16) | ((input[i + 1] as u32) << 8);
            out.push(ALPHABET[((n >> 18) & 63) as usize] as char);
            out.push(ALPHABET[((n >> 12) & 63) as usize] as char);
            out.push(ALPHABET[((n >> 6) & 63) as usize] as char);
        }
        _ => {}
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
            ("mimetype", b"application/nodx+zip".to_vec(), 0o100644),
            (
                "manifest.yaml",
                b"schema: nodx-package/1.0\nentry: a/b.nodx\n".to_vec(),
                0o100644,
            ),
            ("a/b.nodx", b"# A\n".to_vec(), 0o100644),
            (" a/b.nodx ", b"# B\n".to_vec(), 0o100644),
        ]);
        let err = Package::open(&bytes, ResourceLimits::default()).unwrap_err();
        assert!(err.message.contains("Duplicate"));
    }

    #[test]
    fn symlink_entries_are_rejected() {
        let bytes = build_zip(vec![
            ("mimetype", b"application/nodx+zip".to_vec(), 0o100644),
            (
                "manifest.yaml",
                b"schema: nodx-package/1.0\nentry: doc.nodx\n".to_vec(),
                0o100644,
            ),
            ("doc.nodx", b"# A\n".to_vec(), 0o120777),
        ]);
        let err = Package::open(&bytes, ResourceLimits::default()).unwrap_err();
        assert!(err.message.contains("special"));
    }

    #[test]
    fn nested_zip_entries_are_rejected() {
        let bytes = build_zip(vec![
            ("mimetype", b"application/nodx+zip".to_vec(), 0o100644),
            (
                "manifest.yaml",
                b"schema: nodx-package/1.0\nentry: doc.nodx\n".to_vec(),
                0o100644,
            ),
            ("doc.nodx", b"# A\n".to_vec(), 0o100644),
            ("assets/nested.zip", b"PK\x03\x04demo".to_vec(), 0o100644),
        ]);
        let err = Package::open(&bytes, ResourceLimits::default()).unwrap_err();
        assert_eq!(err.code, "NODX-E010");
    }

    fn package_bytes(tamper_digest: bool) -> Vec<u8> {
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
            ("mimetype", b"application/nodx+zip".to_vec(), 0o100644),
            ("manifest.yaml", manifest, 0o100644),
            ("doc.nodx", doc, 0o100644),
        ])
    }

    fn build_zip(entries: Vec<(&str, Vec<u8>, u32)>) -> Vec<u8> {
        let mut out = Vec::new();
        let mut central = Vec::new();
        for (name, data, mode) in entries {
            let local_offset = out.len() as u32;
            let crc = crc32_bytes(&data);
            out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
            out.extend_from_slice(&20u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&crc.to_le_bytes());
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&(name.len() as u16).to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(name.as_bytes());
            out.extend_from_slice(&data);
            central.push((name.to_string(), data.len() as u32, crc, local_offset, mode));
        }
        let cd_offset = out.len() as u32;
        for (name, len, crc, local_offset, mode) in &central {
            out.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
            out.extend_from_slice(&20u16.to_le_bytes());
            out.extend_from_slice(&20u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
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

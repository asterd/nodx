from dataclasses import dataclass
from zlib import crc32

from .block_parser import parse
from .bytes import sha256_base64_url
from .limits import DEFAULT_LIMITS
from .package_diagnostics import raise_diag
from .url import normalize_package_path


@dataclass(frozen=True)
class StoredPackage:
    entry_path: str
    entry_bytes: bytes
    files: dict
    manifest: dict


@dataclass(frozen=True)
class ZipEntry:
    name: str
    compression: int
    crc32: int
    compressed_size: int
    uncompressed_size: int
    local_offset: int
    external_attrs: int


def is_packaged_nodx(bytes_):
    return len(bytes_) >= 4 and bytes_[0:4] == b"PK\x03\x04"


def open_stored_package(bytes_, limits=None):
    limits = limits or DEFAULT_LIMITS
    bytes_ = bytes(bytes_)
    if not is_packaged_nodx(bytes_):
        raise_diag("NODX-E012", "Not a packaged NODX.")
    entries = zip_entries(bytes_, limits)
    if len(entries) > limits["packageFileCount"]:
        raise_diag("NODX-E012", "Package file count limit exceeded.")
    if sum(entry.uncompressed_size for entry in entries) > limits["packageUncompressedBytes"]:
        raise_diag("NODX-E012", "Package uncompressed size limit exceeded.")
    if not entries or entries[0].name != "mimetype" or entries[0].compression != 0:
        raise_diag("NODX-E012", "First ZIP entry must be mimetype.")

    files = {entry.name: read_stored_entry(bytes_, entry, limits) for entry in entries}
    mimetype = files.get("mimetype")
    if not mimetype or decode(mimetype) != "application/nodx+zip":
        raise_diag("NODX-E012", "Invalid NODX package mimetype.")
    manifest_bytes = files.get("manifest.yaml")
    if not manifest_bytes:
        raise_diag("NODX-E012", "Package is missing manifest.yaml.")
    manifest = parse_manifest(decode(manifest_bytes), limits)
    if manifest.get("schema") != "nodx-package/1.0":
        raise_diag("NODX-E012", "Package manifest schema must be `nodx-package/1.0`.")
    if len(manifest["entries"]) > limits["manifestEntries"]:
        raise_diag("NODX-E012", "Package manifest entry limit exceeded.")
    for item in manifest["entries"]:
        data = files.get(item["path"])
        if data is None:
            raise_diag("NODX-E021", "Manifest lists a missing package entry.")
        if item.get("size") is not None and item["size"] != len(data):
            raise_diag("NODX-E021", "Package manifest size does not match entry bytes.")
        if item.get("sha256") and item["sha256"] != sha256_base64_url(data):
            raise_diag("NODX-E021", "Package digest mismatch.")
    for path in manifest_paths(manifest.get("components")) + manifest_paths(manifest.get("themes")):
        if path not in files:
            raise_diag("NODX-E021", "Manifest lists a missing package extension.")
        if not any(entry["path"] == path for entry in manifest["entries"]):
            raise_diag("NODX-E021", "Package extension paths must also be listed in manifest entries.")
    if not manifest.get("entry"):
        raise_diag("NODX-E012", "Package manifest is missing entry.")
    manifest["entry"] = safe_package_path(manifest["entry"], limits)
    if manifest["entry"] not in files:
        raise_diag("NODX-E012", "Package entry document is missing.")
    if manifest.get("signature"):
        manifest["signature"] = safe_package_path(manifest["signature"], limits)
        if manifest["signature"] not in files:
            raise_diag("NODX-E012", "Package manifest references a missing signature.")
    return StoredPackage(manifest["entry"], files[manifest["entry"]], files, manifest)


def package_entry_text(bytes_):
    return decode(open_stored_package(bytes_).entry_bytes)


def parse_packaged_document(bytes_, options=None):
    pkg = open_stored_package(bytes_)
    doc = parse(decode(pkg.entry_bytes))
    return apply_package_extensions(doc, pkg, options)


def apply_package_extensions(doc, pkg, options=None):
    options = options or {}
    out = {**doc, "meta": {**doc["meta"]}}
    out["meta"]["components"] = list(out["meta"].get("components") or [])
    out["meta"]["stylesheets"] = list(out["meta"].get("stylesheets") or [])
    out["meta"]["components"].extend(package_component_definitions(pkg, options))
    out["meta"]["stylesheets"].extend(package_theme_stylesheets(pkg, options))
    if not out["meta"]["components"]:
        del out["meta"]["components"]
    if not out["meta"]["stylesheets"]:
        del out["meta"]["stylesheets"]
    return out


def package_component_definitions(pkg, options=None):
    options = options or {}
    paths = manifest_paths(pkg.manifest.get("components"))
    if options.get("autodiscover_components"):
        for path in pkg.files:
            if path.startswith("components/") and path.endswith(".nodx") and path not in paths:
                paths.append(path)
    return [parse_component_file(decode_required(pkg.files, path), path) for path in paths]


def package_theme_stylesheets(pkg, options=None):
    options = options or {}
    paths = manifest_paths(pkg.manifest.get("themes"))
    if options.get("autodiscover_themes"):
        for path in pkg.files:
            if (path.startswith("themes/") or path.startswith("styles/")) and path.endswith(".nods") and path not in paths:
                paths.append(path)
    return [decode_required(pkg.files, path) for path in paths]


def zip_entries(bytes_, limits):
    eocd = find_eocd(bytes_)
    if eocd < 0:
        raise_diag("NODX-E012", "ZIP end of central directory not found.")
    if u16(bytes_, eocd + 4) != 0 or u16(bytes_, eocd + 6) != 0:
        raise_diag("NODX-E012", "Multi-disk ZIP is not supported.")
    count = u16(bytes_, eocd + 10)
    cd_size = u32(bytes_, eocd + 12)
    cd_offset = u32(bytes_, eocd + 16)
    if count == 0xFFFF or cd_size == 0xFFFFFFFF or cd_offset == 0xFFFFFFFF:
        raise_diag("NODX-E012", "ZIP64 packages are not supported.")

    pos = cd_offset
    entries = []
    seen = set()
    for _ in range(count):
        if u32(bytes_, pos) != 0x02014B50:
            raise_diag("NODX-E012", "Invalid ZIP central directory.")
        flags = u16(bytes_, pos + 8)
        compression = u16(bytes_, pos + 10)
        entry_crc = u32(bytes_, pos + 16)
        compressed_size = u32(bytes_, pos + 20)
        uncompressed_size = u32(bytes_, pos + 24)
        name_len = u16(bytes_, pos + 28)
        extra_len = u16(bytes_, pos + 30)
        comment_len = u16(bytes_, pos + 32)
        external_attrs = u32(bytes_, pos + 38)
        local_offset = u32(bytes_, pos + 42)
        if compressed_size == 0xFFFFFFFF or uncompressed_size == 0xFFFFFFFF or local_offset == 0xFFFFFFFF:
            raise_diag("NODX-E012", "ZIP64 packages are not supported.")
        if uncompressed_size > limits["packageEntryBytes"]:
            raise_diag("NODX-E012", "Package entry size limit exceeded.")
        if compressed_size == 0 and uncompressed_size > 0:
            raise_diag("NODX-E012", "Package compression ratio limit exceeded.")
        if compressed_size > 0 and uncompressed_size > compressed_size * limits["packageCompressionRatio"]:
            raise_diag("NODX-E012", "Package compression ratio limit exceeded.")
        if flags & 1:
            raise_diag("NODX-E012", "Encrypted ZIP entries are not supported.")
        if flags & 8:
            raise_diag("NODX-E012", "ZIP data descriptors are not supported.")
        if compression != 0:
            raise_diag("NODX-E012", "Browser package reader supports stored ZIP entries only.")
        reject_special_file(external_attrs)
        name_start = pos + 46
        name_end = name_start + name_len
        extra_end = name_end + extra_len
        if extra_end > len(bytes_):
            raise_diag("NODX-E012", "ZIP entry name is out of bounds.")
        reject_zip64_extra(bytes_[name_end:extra_end])
        name = safe_package_path(decode(bytes_[name_start:name_end]), limits)
        if name in seen:
            raise_diag("NODX-E012", "Duplicate package entry path.")
        seen.add(name)
        entries.append(ZipEntry(name, compression, entry_crc, compressed_size, uncompressed_size, local_offset, external_attrs))
        pos = extra_end + comment_len
    return sorted(entries, key=lambda entry: entry.local_offset)


def read_stored_entry(bytes_, entry, limits):
    pos = entry.local_offset
    if u32(bytes_, pos) != 0x04034B50:
        raise_diag("NODX-E012", "Invalid ZIP local header.")
    flags = u16(bytes_, pos + 6)
    compression = u16(bytes_, pos + 8)
    entry_crc = u32(bytes_, pos + 14)
    compressed_size = u32(bytes_, pos + 18)
    uncompressed_size = u32(bytes_, pos + 22)
    if flags & 1:
        raise_diag("NODX-E012", "Encrypted ZIP entries are not supported.")
    if flags & 8:
        raise_diag("NODX-E012", "ZIP data descriptors are not supported.")
    if (
        compression != entry.compression
        or entry_crc != entry.crc32
        or compressed_size != entry.compressed_size
        or uncompressed_size != entry.uncompressed_size
    ):
        raise_diag("NODX-E012", "ZIP local header does not match central directory.")
    name_len = u16(bytes_, pos + 26)
    extra_len = u16(bytes_, pos + 28)
    name_start = pos + 30
    name_end = name_start + name_len
    extra_end = name_end + extra_len
    if extra_end > len(bytes_):
        raise_diag("NODX-E012", "ZIP entry data is out of bounds.")
    reject_zip64_extra(bytes_[name_end:extra_end])
    name = safe_package_path(decode(bytes_[name_start:name_end]), limits)
    if name != entry.name:
        raise_diag("NODX-E012", "ZIP local header name mismatch.")
    reject_special_file(entry.external_attrs)
    end = extra_end + entry.compressed_size
    if end > len(bytes_):
        raise_diag("NODX-E012", "ZIP entry data is out of bounds.")
    data = bytes_[extra_end:end]
    if len(data) != entry.uncompressed_size:
        raise_diag("NODX-E012", "Stored ZIP entry has inconsistent sizes.")
    if crc32(data) & 0xFFFFFFFF != entry.crc32:
        raise_diag("NODX-E012", "ZIP CRC mismatch.")
    if looks_like_zip(data):
        raise_diag("NODX-E010", "Nested ZIP archives are not supported.")
    return data


def parse_manifest(text, limits):
    manifest = {"schema": "", "entry": "", "signature": "", "entries": [], "components": [], "themes": []}
    section = ""
    current_entry = None
    seen_top = set()
    for raw in text.replace("\r\n", "\n").split("\n"):
        line = raw.rstrip()
        trimmed = line.strip()
        if not trimmed or trimmed.startswith("#"):
            continue
        if not raw.startswith(" ") and not raw.startswith("-"):
            section = ""
            current_entry = None
            key = trimmed[:-1] if trimmed.endswith(":") else trimmed.split(":", 1)[0]
            if key in seen_top:
                raise_diag("NODX-E012", "Duplicate manifest key.")
            seen_top.add(key)
        if line.startswith("schema:"):
            manifest["schema"] = unquote(line[7:].strip())
        elif line.startswith("entry:"):
            manifest["entry"] = unquote(line[6:].strip())
        elif line.startswith("signature:"):
            manifest["signature"] = unquote(line[10:].strip())
        elif line in ("entries:", "components:", "themes:"):
            section = line[:-1]
        elif section == "entries" and trimmed.startswith("- path:"):
            current_entry = {"path": safe_package_path(unquote(trimmed[7:].strip()), limits), "size": None, "sha256": ""}
            manifest["entries"].append(current_entry)
        elif section == "entries" and current_entry is not None and trimmed.startswith("size:"):
            current_entry["size"] = int(trimmed[5:].strip())
            if current_entry["size"] < 0:
                raise_diag("NODX-E012", "Invalid manifest entry size.")
        elif section == "entries" and current_entry is not None and trimmed.startswith("sha256:"):
            current_entry["sha256"] = unquote(trimmed[7:].strip())
        elif section in ("components", "themes") and trimmed.startswith("- path:"):
            manifest[section].append({"path": safe_package_path(unquote(trimmed[7:].strip()), limits)})
    return manifest


def manifest_paths(items):
    return [item if isinstance(item, str) else item.get("path") for item in items or [] if (item if isinstance(item, str) else item.get("path"))]


def parse_component_file(text, path):
    body = strip_front_matter(text)
    parsed = parse(text)
    name = parsed["meta"].get("name") if isinstance(parsed["meta"].get("name"), str) else path.rsplit("/", 1)[-1].removesuffix(".nodx")
    component = {"name": name, "template": body}
    if isinstance(parsed["meta"].get("style"), str) and parsed["meta"]["style"].strip():
        component["style"] = parsed["meta"]["style"]
    return component


def strip_front_matter(text):
    if not text.startswith("---\n"):
        return text
    end = text.find("\n---", 4)
    if end < 0:
        return text
    body_start = end + 4
    if body_start < len(text) and text[body_start] == "\r":
        body_start += 1
    if body_start < len(text) and text[body_start] == "\n":
        body_start += 1
    return text[body_start:]


def safe_package_path(path, limits):
    normalized = normalize_package_path(path, limits)
    if normalized is None:
        raise_diag("NODX-E010", f"Unsafe package path: {path}")
    return normalized


def reject_zip64_extra(extra):
    pos = 0
    while pos + 4 <= len(extra):
        header = int.from_bytes(extra[pos:pos + 2], "little")
        size = int.from_bytes(extra[pos + 2:pos + 4], "little")
        pos += 4
        end = pos + size
        if end > len(extra):
            raise_diag("NODX-E012", "Invalid ZIP extra field.")
        if header == 0x0001:
            raise_diag("NODX-E012", "ZIP64 packages are not supported.")
        pos = end
    if pos != len(extra):
        raise_diag("NODX-E012", "Invalid ZIP extra field.")


def reject_special_file(external_attrs):
    mode = external_attrs >> 16
    if mode == 0:
        return
    if mode & 0o170000 != 0o100000:
        raise_diag("NODX-E012", "ZIP special files are not supported.")


def looks_like_zip(data):
    return len(data) >= 4 and data[0:2] == b"PK" and data[2] in (0x03, 0x05, 0x07)


def find_eocd(bytes_):
    start = max(0, len(bytes_) - 65557)
    for pos in range(len(bytes_) - 22, start - 1, -1):
        if bytes_[pos:pos + 4] == b"PK\x05\x06":
            return pos
    return -1


def decode_required(files, path):
    if path not in files:
        raise_diag("NODX-E021", f"Package manifest references missing path: {path}")
    return decode(files[path])


def unquote(value):
    if (value.startswith('"') and value.endswith('"')) or (value.startswith("'") and value.endswith("'")):
        return value[1:-1]
    return value


def u16(bytes_, pos):
    if pos + 2 > len(bytes_):
        raise_diag("NODX-E012", "Unexpected end of ZIP data.")
    return int.from_bytes(bytes_[pos:pos + 2], "little")


def u32(bytes_, pos):
    if pos + 4 > len(bytes_):
        raise_diag("NODX-E012", "Unexpected end of ZIP data.")
    return int.from_bytes(bytes_[pos:pos + 4], "little")


def decode(bytes_):
    return bytes_.decode("utf-8", errors="strict")


isPackagedNodx = is_packaged_nodx
openStoredPackage = open_stored_package
packageEntryText = package_entry_text
parsePackagedDocument = parse_packaged_document
applyPackageExtensions = apply_package_extensions
packageComponentDefinitions = package_component_definitions
packageThemeStylesheets = package_theme_stylesheets

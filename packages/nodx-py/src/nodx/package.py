import zipfile
from dataclasses import dataclass
from io import BytesIO


@dataclass(frozen=True)
class StoredPackage:
    entry_path: str
    entry_bytes: bytes
    files: dict
    manifest: dict


def is_packaged_nodx(bytes_):
    return len(bytes_) >= 4 and bytes_[0:4] == b"PK\x03\x04"


def open_stored_package(bytes_):
    if not isinstance(bytes_, (bytes, bytearray, memoryview)):
        bytes_ = bytes(bytes_)
    bytes_ = bytes(bytes_)
    if not is_packaged_nodx(bytes_):
        raise ValueError("not a packaged NODX")
    try:
        archive = zipfile.ZipFile(BytesIO(bytes_))
    except zipfile.BadZipFile as exc:
        raise ValueError("ZIP end of central directory not found") from exc
    files = {}
    for info in archive.infolist():
        if info.flag_bits & 1:
            raise ValueError("encrypted ZIP entries are not supported")
        if info.compress_type != zipfile.ZIP_STORED:
            raise ValueError("browser package reader supports stored ZIP entries only")
        if not is_safe_package_path(info.filename):
            raise ValueError("unsafe package path: " + info.filename)
        files[info.filename] = archive.read(info)
    mimetype = files.get("mimetype")
    if not mimetype or decode(mimetype) != "application/nodx+zip":
        raise ValueError("invalid NODX package mimetype")
    manifest_bytes = files.get("manifest.yaml")
    if not manifest_bytes:
        raise ValueError("package is missing manifest.yaml")
    manifest = parse_manifest(decode(manifest_bytes))
    if manifest.get("schema") != "nodx-package/1.0":
        raise ValueError("package manifest schema must be nodx-package/1.0")
    if not manifest.get("entry"):
        raise ValueError("package manifest is missing entry")
    if manifest["entry"] not in files:
        raise ValueError("package entry document is missing")
    return StoredPackage(manifest["entry"], files[manifest["entry"]], files, manifest)


def package_entry_text(bytes_):
    return decode(open_stored_package(bytes_).entry_bytes)


def parse_manifest(text):
    manifest = {"schema": "", "entry": "", "signature": "", "entries": []}
    in_entries = False
    for raw in text.replace("\r\n", "\n").split("\n"):
        line = raw.rstrip()
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        if not raw.startswith(" ") and not raw.startswith("-"):
            in_entries = False
        if line.startswith("schema:"):
            manifest["schema"] = unquote(line[7:].strip())
        elif line.startswith("entry:"):
            manifest["entry"] = unquote(line[6:].strip())
        elif line.startswith("signature:"):
            manifest["signature"] = unquote(line[10:].strip())
        elif line == "entries:":
            in_entries = True
        elif in_entries and line.lstrip().startswith("- path:"):
            manifest["entries"].append({"path": unquote(line.lstrip()[7:].strip())})
    return manifest


def unquote(value):
    if (value.startswith('"') and value.endswith('"')) or (
        value.startswith("'") and value.endswith("'")
    ):
        return value[1:-1]
    return value


def is_safe_package_path(path):
    if not path or path.startswith("/") or "\\" in path or "//" in path:
        return False
    return all(part and part not in (".", "..") for part in path.split("/"))


def decode(bytes_):
    return bytes_.decode("utf-8", errors="strict")


isPackagedNodx = is_packaged_nodx
openStoredPackage = open_stored_package
packageEntryText = package_entry_text

import zipfile
from dataclasses import dataclass
from io import BytesIO

from .block_parser import parse


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


def parse_manifest(text):
    manifest = {"schema": "", "entry": "", "signature": "", "entries": [], "components": [], "themes": []}
    in_entries = False
    list_field = None
    for raw in text.replace("\r\n", "\n").split("\n"):
        line = raw.rstrip()
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        if not raw.startswith(" ") and not raw.startswith("-"):
            in_entries = False
            list_field = None
        if line.startswith("schema:"):
            manifest["schema"] = unquote(line[7:].strip())
        elif line.startswith("entry:"):
            manifest["entry"] = unquote(line[6:].strip())
        elif line.startswith("signature:"):
            manifest["signature"] = unquote(line[10:].strip())
        elif line == "entries:":
            in_entries = True
        elif line == "components:":
            list_field = "components"
        elif line == "themes:":
            list_field = "themes"
        elif in_entries and line.lstrip().startswith("- path:"):
            manifest["entries"].append({"path": unquote(line.lstrip()[7:].strip())})
        elif list_field and line.lstrip().startswith("- path:"):
            manifest[list_field].append({"path": unquote(line.lstrip()[7:].strip())})
    return manifest


def manifest_paths(items):
    return [item if isinstance(item, str) else item.get("path") for item in items or [] if (item if isinstance(item, str) else item.get("path"))]


def decode_required(files, path):
    if path not in files:
        raise ValueError("package manifest references missing path: " + path)
    return decode(files[path])


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
parsePackagedDocument = parse_packaged_document
applyPackageExtensions = apply_package_extensions
packageComponentDefinitions = package_component_definitions
packageThemeStylesheets = package_theme_stylesheets

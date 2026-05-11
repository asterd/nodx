from enum import Enum

from .limits import DEFAULT_LIMITS


class ReferenceKind(str, Enum):
    Link = "link"
    Asset = "asset"
    Style = "style"
    Include = "include"
    Font = "font"
    MediaFallback = "media-fallback"


FORBIDDEN_SCHEMES = {"javascript", "vbscript", "file", "jar", "chrome", "about"}
ALLOWED_DATA_MIMES = {"image/png", "image/jpeg", "image/webp", "image/gif"}


def classify_uri(kind, raw, limits=None):
    limits = limits or DEFAULT_LIMITS
    trimmed = trim_ascii_whitespace(raw)
    if len(trimmed) == 0:
        return {"ok": False, "error": "empty"}
    if len(trimmed) > limits["urlBytes"]:
        return {"ok": False, "error": "too-long"}
    ctrl = reject_control_or_backslash(trimmed)
    if ctrl is not None:
        return {"ok": False, "error": ctrl}
    kind_value = kind.value if isinstance(kind, ReferenceKind) else kind
    if trimmed.startswith("#"):
        if kind_value == ReferenceKind.Link.value:
            return {"ok": True, "class": {"type": "fragment", "value": trimmed}}
        return {"ok": False, "error": "invalid-package-path"}
    scheme = scheme_prefix(trimmed)
    if scheme == "error":
        return {"ok": False, "error": "unsafe-scheme"}
    if scheme is not None:
        return classify_scheme(kind_value, trimmed, scheme, limits)
    path = normalize_package_path(trimmed, limits)
    if path is None:
        return {"ok": False, "error": "invalid-package-path"}
    return {"ok": True, "class": {"type": "package-relative", "value": path}}


def normalize_package_path(raw, limits=None):
    limits = limits or DEFAULT_LIMITS
    trimmed = trim_ascii_whitespace(raw)
    if len(trimmed) == 0 or len(trimmed) > limits["urlBytes"]:
        return None
    if reject_control_or_backslash(trimmed) is not None:
        return None
    if trimmed.startswith("/") or scheme_prefix(trimmed) is not None:
        return None
    if len(trimmed) > limits["packagePathBytes"]:
        return None
    parts = trimmed.split("/")
    if len(parts) > limits["packagePathSegments"]:
        return None
    normalized = []
    for part in parts:
        if part in ("", "."):
            return None
        decoded = percent_decode_ascii(part)
        if decoded is None or part == ".." or decoded == "..":
            return None
        if ":" in decoded or "/" in decoded or "\\" in decoded or has_control_char(decoded):
            return None
        normalized.append(part)
    return "/".join(normalized)


def classify_scheme(kind, raw, scheme, limits):
    if scheme in FORBIDDEN_SCHEMES:
        return {"ok": False, "error": "unsafe-scheme"}
    if kind == ReferenceKind.Link.value and scheme in ("http", "https", "mailto", "tel"):
        return {"ok": True, "class": {"type": "absolute", "value": raw, "scheme": scheme}}
    if kind == ReferenceKind.Asset.value and scheme == "data":
        mime = validate_data_uri(raw, limits)
        if mime is None:
            return {"ok": False, "error": "unsafe-data"}
        return {"ok": True, "class": {"type": "data", "value": raw, "mime": mime}}
    return {"ok": False, "error": "unsafe-scheme"}


def reject_control_or_backslash(input_):
    if "\\" in input_:
        return "backslash"
    if has_control_char(input_):
        return "control"
    return None


def has_control_char(input_):
    return any(ord(ch) < 0x20 or ord(ch) == 0x7F for ch in input_)


def scheme_prefix(input_):
    boundary = len(input_)
    for i, ch in enumerate(input_):
        if ch in ":/?#":
            boundary = i
            break
    if boundary >= len(input_) or input_[boundary] != ":":
        return None
    if boundary == 0:
        return "error"
    decoded = percent_decode_ascii(input_[:boundary])
    if decoded is None:
        return "error"
    lower = decoded.lower()
    if not lower or any(not is_scheme_char(ch) for ch in lower):
        return "error"
    return lower


def is_scheme_char(ch):
    return ("a" <= ch <= "z") or ("0" <= ch <= "9") or ch in "+-."


def validate_data_uri(raw, limits):
    if len(raw) > limits["dataUriBytes"]:
        return None
    colon = raw.find(":")
    if colon < 0 or raw[:colon].lower() != "data":
        return None
    rest = raw[colon + 1 :]
    comma = rest.find(",")
    if comma < 0:
        return None
    mime = (rest[:comma].split(";")[0] or "").strip().lower()
    return mime if mime in ALLOWED_DATA_MIMES else None


def percent_decode_ascii(input_):
    out = ""
    i = 0
    while i < len(input_):
        if input_[i] == "%":
            if i + 2 >= len(input_):
                return None
            try:
                v = int(input_[i + 1 : i + 3], 16)
            except ValueError:
                return None
            if v >= 0x80:
                return None
            out += chr(v)
            i += 3
        else:
            out += input_[i]
            i += 1
    return out


def trim_ascii_whitespace(input_):
    return input_.strip("\t\n\f\r ")

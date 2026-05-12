from .ast import empty_attrs


def parse_attrs(raw):
    attrs = empty_attrs()
    s = raw.strip()
    if not (s.startswith("{") and s.endswith("}")):
        return attrs
    for token in split_attrs(s[1:-1]):
        if token.startswith("#"):
            attrs["id"] = token[1:]
        elif token.startswith("."):
            attrs["classes"].append(token[1:])
        else:
            i = token.find("=")
            if i > 0:
                apply_attr(attrs, token[:i], unquote(token[i + 1 :]))
            elif token == "highlight":
                attrs["styles"]["background-color"] = "color-mix(in srgb, var(--nodx-color-accent) 14%, transparent)"
                attrs["styles"]["padding"] = "0.05em 0.25em"
                attrs["styles"]["border-radius"] = "0.2em"
    attrs["classes"] = sorted(set(attrs["classes"]))
    if not attrs["styles"]:
        del attrs["styles"]
    return attrs


def merge_class_suffix(attrs, suffix):
    for cls in parse_class_suffix(suffix):
        attrs["classes"].append(cls)
    attrs["classes"] = sorted(set(attrs["classes"]))
    return attrs


def parse_class_suffix(input_):
    import re

    classes = []
    i = 0
    while i < len(input_) and input_[i] == ".":
        match = re.match(r"^[A-Za-z_][A-Za-z0-9_-]*", input_[i + 1 :])
        if not match:
            break
        classes.append(match.group(0))
        i += len(match.group(0)) + 1
    return classes


STYLE_SHORTHANDS = {
    "bg": "background-color",
    "color": "color",
    "border": "border",
    "radius": "border-radius",
    "pad": "padding",
    "padding": "padding",
    "font": "font",
    "weight": "font-weight",
}
ALLOWED_INLINE_PROPERTIES = {"background-color", "border-radius", "font-weight"}


def apply_attr(attrs, key, value):
    if key == "class":
        attrs["classes"].extend([item for item in value.split() if item])
        return
    prop = STYLE_SHORTHANDS.get(key) or (key if key in ALLOWED_INLINE_PROPERTIES else None)
    if prop and safe_inline_style_value(value):
        attrs["styles"][prop] = value
        return
    attrs["attrs"][key] = value


def safe_inline_style_value(value):
    lower = value.lower()
    return (
        len(value) <= 240
        and not any(ch in value for ch in "<>{};")
        and "expression(" not in lower
        and "javascript:" not in lower
        and "vbscript:" not in lower
        and "@import" not in lower
        and "url(" not in lower
    )


def split_attrs(input_):
    out = []
    buf = ""
    quoted = False
    for ch in input_:
        if ch == '"':
            quoted = not quoted
        if ch == " " and not quoted:
            if buf:
                out.append(buf)
            buf = ""
        else:
            buf += ch
    if buf:
        out.append(buf)
    return out


def unquote(raw):
    return raw[1:-1] if is_quoted(raw) else raw


def is_quoted(raw):
    return (raw.startswith('"') and raw.endswith('"')) or (
        raw.startswith("'") and raw.endswith("'")
    )

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
                attrs["attrs"][token[:i]] = unquote(token[i + 1 :])
    attrs["classes"] = sorted(set(attrs["classes"]))
    return attrs


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

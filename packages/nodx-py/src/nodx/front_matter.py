import re

from .attrs import is_quoted, unquote
from .diagnostics import diag


def parse_meta(lines, diagnostics):
    meta, _ = parse_block_mapping(lines, 0, 0, diagnostics, True)
    return meta


def check_yaml_safety(line, line_no, diagnostics, inside_block_scalar=False):
    if inside_block_scalar:
        return
    trimmed = line.lstrip()
    unquoted = re.sub(r'"[^"]*"|\'[^\']*\'', "", trimmed)
    if trimmed in ("---", "...") or trimmed.startswith("--- ") or trimmed.startswith("... "):
        diagnostics.append(diag("NODX-E019", "fatal", "Multiple YAML documents are not supported.", line_no, 1))
    if any(item in unquoted for item in ("&", "*", "!", "<<:")) or trimmed.startswith("? "):
        diagnostics.append(diag("NODX-E019", "fatal", "Forbidden YAML safe-subset construct.", line_no, 1))
    if re.match(r"^\s*\t", line):
        diagnostics.append(diag("NODX-E019", "fatal", "Forbidden YAML indentation.", line_no, 1))
    for ch in line:
        code = ord(ch)
        if (code < 0x20 and code not in (0x09, 0x0A, 0x0D)) or code == 0x7F:
            diagnostics.append(diag("NODX-E019", "fatal", "Forbidden control character in YAML front matter.", line_no, 1))
            break
    idx = trimmed.find(":")
    value = trimmed[idx + 1 :].strip() if idx >= 0 else re.sub(r"^- ", "", trimmed).strip()
    key = trimmed[:idx].strip() if idx >= 0 else ""
    if key in ("<<", "?") or key.startswith("[") or key.startswith("{"):
        diagnostics.append(diag("NODX-E019", "fatal", "Forbidden YAML mapping key.", line_no, 1))
    if value.startswith("[") or value.startswith("{"):
        diagnostics.append(diag("NODX-E019", "fatal", "Flow-style YAML collections are not allowed.", line_no, 1))
    if not is_quoted(value):
        lower = value.lower()
        if lower in (".nan", ".inf", "+.inf", "-.inf", ".infinity", "+.infinity", "-.infinity"):
            diagnostics.append(diag("NODX-E019", "fatal", "Forbidden YAML non-finite number.", line_no, 1))
        if re.match(r"^[+-]?0[box]", value, re.I):
            diagnostics.append(diag("NODX-E019", "fatal", "Forbidden YAML numeric special.", line_no, 1))
        if lower in ("yes", "no", "on", "off", "y", "n"):
            diagnostics.append(diag("NODX-E019", "fatal", "Forbidden YAML boolean alias; use true/false.", line_no, 1))
        if re.match(r"^\d{4}-\d{2}-\d{2}", value):
            diagnostics.append(diag("NODX-E019", "fatal", "Native YAML timestamps are not supported.", line_no, 1))


def line_indent(line):
    return len(line) - len(line.lstrip(" "))


def parse_block_mapping(lines, start, indent, diagnostics, top):
    map_ = {}
    i = start
    while i < len(lines):
        line = lines[i]
        if line.strip() == "":
            i += 1
            continue
        li = line_indent(line)
        if li < indent:
            break
        if li > indent:
            i += 1
            continue
        check_yaml_safety(line, i + 2, diagnostics)
        trimmed = line[li:]
        if trimmed.startswith("- ") or trimmed == "-":
            break
        idx = trimmed.find(":")
        if idx < 0:
            i += 1
            continue
        key = trimmed[:idx].strip()
        rest = trimmed[idx + 1 :].strip()
        if key in map_:
            diagnostics.append(diag("NODX-E019", "fatal", "Duplicate front matter key.", i + 2, 1))
        if rest != "":
            if is_block_scalar(rest):
                map_[key], i = parse_block_scalar(lines, i + 1, indent)
            else:
                map_[key] = scalar(rest)
                i += 1
            continue
        next_ = lines[i + 1] if i + 1 < len(lines) else ""
        next_indent = line_indent(next_)
        if next_.strip() == "" or next_indent <= indent:
            map_[key] = ""
            i += 1
            continue
        next_trim = next_[next_indent:]
        if next_trim.startswith("- ") or next_trim == "-":
            list_, consumed = parse_block_sequence(lines, i + 1, next_indent, diagnostics)
            map_[key] = list_
            i = consumed
            continue
        child, consumed = parse_block_mapping(lines, i + 1, next_indent, diagnostics, False)
        map_[key] = child
        i = consumed
    return map_, i


def parse_block_sequence(lines, start, indent, diagnostics):
    out = []
    i = start
    while i < len(lines):
        line = lines[i]
        if line.strip() == "":
            i += 1
            continue
        li = line_indent(line)
        if li != indent:
            break
        trimmed = line[li:]
        if not (trimmed.startswith("- ") or trimmed == "-"):
            break
        check_yaml_safety(line, i + 2, diagnostics)
        after = "" if trimmed == "-" else trimmed[2:]
        colon = after.find(":")
        if colon >= 0 and (colon == 0 or re.search(r"[A-Za-z0-9_-]$", after[:colon].strip())):
            key = after[:colon].strip()
            rest = after[colon + 1 :].strip()
            child = {}
            i += 1
            if rest != "":
                if is_block_scalar(rest):
                    child[key], i = parse_block_scalar(lines, i, indent)
                else:
                    child[key] = scalar(rest)
            else:
                i = parse_nested_field(lines, i, indent, diagnostics, child, key)
            while i < len(lines):
                peek = lines[i]
                if peek.strip() == "":
                    i += 1
                    continue
                pi = line_indent(peek)
                if pi <= indent:
                    break
                peek_trim = peek[pi:]
                if peek_trim.startswith("- ") or peek_trim == "-":
                    break
                k = peek_trim.find(":")
                if k < 0:
                    break
                check_yaml_safety(peek, i + 2, diagnostics)
                ck = peek_trim[:k].strip()
                cv = peek_trim[k + 1 :].strip()
                if cv != "":
                    if is_block_scalar(cv):
                        child[ck], i = parse_block_scalar(lines, i + 1, pi)
                    else:
                        child[ck] = scalar(cv)
                        i += 1
                else:
                    i += 1
                    i = parse_nested_field(lines, i, pi, diagnostics, child, ck)
            out.append(child)
        else:
            out.append(scalar(after))
            i += 1
    return out, i


def is_block_scalar(raw):
    return raw in ("|", "|-", "|+")


def parse_block_scalar(lines, start, parent_indent):
    i = start
    block_indent = None
    out = []
    while i < len(lines):
        line = lines[i]
        if line.strip() == "":
            out.append("")
            i += 1
            continue
        indent = line_indent(line)
        if indent <= parent_indent:
            break
        if block_indent is None:
            block_indent = indent
        out.append(line[min(block_indent, len(line)) :])
        i += 1
    return "\n".join(out), i


def parse_nested_field(lines, i, indent, diagnostics, child, key):
    if i < len(lines) and lines[i].strip() != "":
        ni = line_indent(lines[i])
        if ni > indent:
            nt = lines[i][ni:]
            if nt.startswith("- ") or nt == "-":
                nested, consumed = parse_block_sequence(lines, i, ni, diagnostics)
            else:
                nested, consumed = parse_block_mapping(lines, i, ni, diagnostics, False)
            child[key] = nested
            return consumed
    return i


def scalar(raw):
    if raw == "null":
        return None
    if raw == "true":
        return True
    if raw == "false":
        return False
    if raw.startswith("[") and raw.endswith("]"):
        return [scalar(item.strip()) for item in raw[1:-1].split(",") if item.strip()]
    number = _canonical_number(raw)
    if number is not None:
        return number
    return unquote(raw)


def _canonical_number(raw):
    """Collapse integer-valued floats to int so the canonical JSON matches
    Rust's `f64::to_string()` (`1.0` -> `1`) and JS's `JSON.stringify(Number)`.

    The regex restricts input to `[+-]?digits[.digits]?`, so `float(raw)`
    cannot raise and cannot produce non-finite values.
    """
    if not re.match(r"^[+-]?\d+(\.\d+)?$", raw):
        return None
    value = float(raw)
    return int(value) if value.is_integer() else value

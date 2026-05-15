import re

from .ast import empty_attrs, node
from .attrs import parse_attrs
from .diagnostics import diag
from .front_matter import parse_meta
from .inline_parser import parse_inlines
from .limits import DEFAULT_LIMITS


def parse(input_, limits=None):
    if limits is None:
        limits = DEFAULT_LIMITS
    diagnostics = []
    if input_.startswith("\ufeff"):
        diagnostics.append(diag("NODX-E018", "fatal", "Byte Order Mark is not allowed.", 1, 1))
    if "\u0000" in input_:
        diagnostics.append(diag("NODX-E002", "fatal", "U+0000 is not allowed.", 1, 1))
    # Text-side resource limits — parity with `nodx_core::parse_str_with_limits`.
    # See packages/nodx-js/src/blockParser.mjs for the JS twin.
    byte_len = len(input_.encode("utf-8"))
    if byte_len > limits["sourceBytes"]:
        diagnostics.append(diag("NODX-E012", "fatal", "Input byte size limit exceeded.", 1, 1))
    oversize = _first_oversize_line(input_, limits["lineLength"])
    if oversize is not None:
        diagnostics.append(diag("NODX-E012", "fatal", "Line length limit exceeded.", oversize, 1))
    fm_bytes = _front_matter_byte_size(input_)
    if fm_bytes is not None and fm_bytes > limits["frontMatterBytes"]:
        diagnostics.append(diag("NODX-E012", "fatal", "Front matter size limit exceeded.", 1, 1))
    if any(d["severity"] == "fatal" for d in diagnostics):
        # Fatal short-circuit: empty Document with baseline meta. Same field
        # set the normal path below produces via setdefault().
        return {
            "schema": "nodx/1.0",
            "meta": {"schema": "nodx/1.0", "type": "document", "dir": "auto", "language": "und"},
            "body": [],
            "diagnostics": diagnostics,
        }
    lines = re.sub(r"\r\n", "\n", input_)
    if lines.endswith("\n"):
        lines = lines[:-1]
    lines = lines.split("\n")
    meta = {}
    start = 0
    had_front_matter = False
    if lines[0] == "---":
        try:
            end = lines.index("---", 1)
        except ValueError:
            diagnostics.append(diag("NODX-E003", "fatal", "Unclosed front matter.", 1, 1))
        else:
            had_front_matter = True
            meta.update(parse_meta(lines[1:end], diagnostics))
            start = end + 1
    meta.setdefault("schema", "nodx/1.0")
    meta.setdefault("type", "document")
    meta.setdefault("dir", "auto")
    meta.setdefault("language", "und")
    if not had_front_matter:
        meta.setdefault("profiles", {"requires": ["core"]})
    state = {"lines": lines, "pos": start, "diagnostics": diagnostics}
    body = parse_until(state, None)
    return {"body": body, "diagnostics": diagnostics, "meta": meta, "schema": "nodx/1.0"}


def parse_until(state, close_frame):
    out = []
    closed = close_frame is None
    while state["pos"] < len(state["lines"]):
        line = state["lines"][state["pos"]]
        if close_frame is not None:
            close = parse_matching_close(line, close_frame["colons"], close_frame["name"])
            if close:
                if close["name"] is not None and close["name"] != close_frame["name"]:
                    state["diagnostics"].append(diag("NODX-E005", "error", "Closing label `" + close["name"] + "` does not match open block `" + close_frame["name"] + "`.", state["pos"] + 1, 1))
                state["pos"] += 1
                closed = True
                break
        elif is_any_close(line):
            state["diagnostics"].append(diag("NODX-E005", "error", "Unmatched block closer.", state["pos"] + 1, 1))
            state["pos"] += 1
            continue
        if line.strip() == "":
            state["pos"] += 1
            continue
        if is_thematic_break(line):
            # Thematic break: leaf node with no children/inlines/text. The
            # opening front matter `---` is consumed above, so by here `---`
            # is unambiguous.
            state["pos"] += 1
            out.append(node("hr", empty_attrs(), [], [], None))
            continue
        opener = parse_opener(line)
        if opener:
            out.append(parse_delimited(state, opener))
            continue
        heading = parse_heading(line)
        if heading:
            state["pos"] += 1
            heading["attrs"]["attrs"]["level"] = str(heading["level"])
            out.append(node("heading", heading["attrs"], [], parse_inlines(heading["content"]), None))
            continue
        if list_kind(line):
            out.append(parse_list(state))
            continue
        if state["pos"] + 1 < len(state["lines"]) and is_pipe_header(line, state["lines"][state["pos"] + 1]):
            out.append(parse_table(state))
            continue
        out.append(parse_paragraph(state))
    if not closed:
        state["diagnostics"].append(diag("NODX-E005", "error", "Unclosed delimited block at end of input.", max(state["pos"], 1), 1))
    return out


def parse_delimited(state, opener):
    state["pos"] += 1
    if opener["name"] in ("code", "pre", "math", "style"):
        start = state["pos"]
        while state["pos"] < len(state["lines"]) and parse_matching_close(state["lines"][state["pos"]], opener["colons"], opener["name"]) is None:
            state["pos"] += 1
        text = "\n".join(state["lines"][start : state["pos"]])
        if state["pos"] < len(state["lines"]):
            close = parse_matching_close(state["lines"][state["pos"]], opener["colons"], opener["name"])
            if close and close["name"] is not None and close["name"] != opener["name"]:
                state["diagnostics"].append(diag("NODX-E005", "error", "Closing label `" + close["name"] + "` does not match open block `" + opener["name"] + "`.", state["pos"] + 1, 1))
            state["pos"] += 1
        else:
            state["diagnostics"].append(diag("NODX-E005", "error", "Unclosed literal block.", start + 1, 1))
        return node(opener["name"], opener["attrs"], [], [], text)
    return node(opener["name"], opener["attrs"], parse_until(state, {"colons": opener["colons"], "name": opener["name"]}), [], None)


def parse_list(state):
    kind = list_kind(state["lines"][state["pos"]])
    children = []
    while state["pos"] < len(state["lines"]) and list_kind(state["lines"][state["pos"]]) == kind:
        content, checked = strip_list(state["lines"][state["pos"]])
        state["pos"] += 1
        parts = [content]
        while state["pos"] < len(state["lines"]) and state["lines"][state["pos"]].startswith("  "):
            parts.append(state["lines"][state["pos"]].lstrip())
            state["pos"] += 1
        attrs = empty_attrs()
        if checked is not None:
            attrs["attrs"]["checked"] = str(checked).lower()
        children.append(node("item", attrs, [], parse_inlines("\n".join(parts)), None))
    attrs = empty_attrs()
    attrs["attrs"]["kind"] = kind
    return node("list", attrs, children, [], None)


def parse_table(state):
    aligns = split_pipe_alignments(state["lines"][state["pos"] + 1])
    rows = [table_row(split_pipe(state["lines"][state["pos"]]), True, aligns)]
    state["pos"] += 2
    while state["pos"] < len(state["lines"]) and "|" in state["lines"][state["pos"]] and state["lines"][state["pos"]].strip():
        rows.append(table_row(split_pipe(state["lines"][state["pos"]]), False, aligns))
        state["pos"] += 1
    return node("table", empty_attrs(), rows, [], None)


def parse_paragraph(state):
    start = state["pos"]
    state["pos"] += 1
    while (
        state["pos"] < len(state["lines"])
        and state["lines"][state["pos"]].strip() != ""
        and not parse_opener(state["lines"][state["pos"]])
        and not parse_heading(state["lines"][state["pos"]])
        and not list_kind(state["lines"][state["pos"]])
        and not is_thematic_break(state["lines"][state["pos"]])
        and not is_any_close(state["lines"][state["pos"]])
    ):
        if state["pos"] + 1 < len(state["lines"]) and is_pipe_header(state["lines"][state["pos"]], state["lines"][state["pos"] + 1]):
            break
        state["pos"] += 1
    return node("paragraph", empty_attrs(), [], parse_inlines("\n".join(state["lines"][start : state["pos"]])), None)


def parse_opener(line):
    match = re.match(r"^(::+)([A-Za-z][A-Za-z0-9-]*)(?:\s+(\{.*\}))?$", line)
    if not match or len(match.group(1)) < 2:
        return None
    return {"colons": len(match.group(1)), "name": match.group(2), "attrs": parse_attrs(match.group(3) or "")}


def parse_heading(line):
    match = re.match(r"^(#{1,6}) (.*)$", line)
    if not match:
        return None
    content = match.group(2).rstrip()
    attrs = empty_attrs()
    attr_start = content.rfind(" {")
    if attr_start >= 0 and content.endswith("}"):
        attrs = parse_attrs(content[attr_start + 1 :])
        content = content[:attr_start].rstrip()
    else:
        light_id = re.match(r"^(.*) (#([A-Za-z][A-Za-z0-9-]*))$", content)
        if light_id:
            content = light_id.group(1).rstrip()
            attrs["id"] = light_id.group(3)
    return {"level": len(match.group(1)), "content": content, "attrs": attrs}


def parse_close(line, n):
    colons = len(line) - len(line.lstrip(":"))
    if colons != n:
        return None
    after = line[n:]
    if after.strip() == "":
        return {"name": None}
    if after.startswith(" "):
        name = after[1:].rstrip()
        if re.match(r"^[A-Za-z][A-Za-z0-9-]*$", name):
            return {"name": name}
    return None


def parse_matching_close(line, n, expected_name):
    close = parse_close(line, n)
    if close:
        return close
    return {"name": expected_name} if line == ":" * n + expected_name else None


def is_thematic_break(line):
    """Mirrors ``nodx_core::block_parser::is_thematic_break``.

    A thematic break is a line whose trimmed content is three or more
    repetitions of a single marker char ``-``, ``*``, or ``_`` with no
    internal whitespace.
    """
    t = line.strip()
    if len(t) < 3:
        return False
    c = t[0]
    if c not in ("-", "*", "_"):
        return False
    return all(ch == c for ch in t)


def is_any_close(line):
    colons = len(line) - len(line.lstrip(":"))
    if colons < 2:
        return False
    after = line[colons:]
    if after.strip() == "":
        return True
    if after.startswith(" "):
        name = after[1:].rstrip()
        return re.match(r"^[A-Za-z][A-Za-z0-9-]*$", name) is not None
    return False


def list_kind(line):
    """Classify a list item header. PR2 (RFC §10.3) accepts the unordered
    markers ``-``, ``*``, ``+`` and the ordered separators ``.`` and ``)``.

    The literal marker is *not* preserved in the AST: only ``kind`` survives,
    keeping the canonical form byte-stable across the new marker variants.
    """
    if line.startswith("- [ ] ") or line.startswith("- [x] "):
        return "task"
    if line.startswith("- ") or line.startswith("* ") or line.startswith("+ "):
        return "unordered"
    if re.match(r"^\d+[.)] ", line):
        return "ordered"
    return None


def strip_list(line):
    if line.startswith("- [ ] "):
        return line[6:], False
    if line.startswith("- [x] "):
        return line[6:], True
    if line.startswith("- ") or line.startswith("* ") or line.startswith("+ "):
        return line[2:], None
    return re.sub(r"^\d+[.)] ", "", line), None


def is_pipe_header(a, b):
    return "|" in a and "-" in b and re.match(r"^[|:\- ]+$", b.strip()) is not None


def split_pipe(line):
    return [part.strip() for part in line.strip().strip("|").split("|")]


def split_pipe_alignments(line):
    out = []
    for cell in split_pipe(line):
        left = cell.startswith(":")
        right = cell.endswith(":")
        if left and right:
            out.append("center")
        elif left:
            out.append("left")
        elif right:
            out.append("right")
        else:
            out.append(None)
    return out


def table_row(cells, header, aligns):
    children = []
    for index, cell in enumerate(cells):
        attrs, content = parse_pipe_cell_attrs(cell)
        if header:
            attrs["attrs"]["header"] = "true"
            attrs["attrs"]["scope"] = "col"
        if index < len(aligns) and aligns[index] and "align" not in attrs["attrs"]:
            attrs["attrs"]["align"] = aligns[index]
        children.append(node("cell", attrs, [], parse_inlines(content), None))
    return node("row", empty_attrs(), children, [], None)


def parse_pipe_cell_attrs(cell):
    trimmed = cell.lstrip()
    if not trimmed.startswith("{"):
        return empty_attrs(), cell
    end = trimmed.find("}")
    if end < 0:
        return empty_attrs(), cell
    after = trimmed[end + 1 :]
    if after and not after.startswith(" "):
        return empty_attrs(), cell
    attrs = parse_attrs(trimmed[: end + 1])
    return (empty_attrs(), cell) if attrs_are_empty(attrs) else (attrs, after.lstrip())


def attrs_are_empty(attrs):
    return not attrs.get("id") and not attrs.get("classes") and not attrs.get("attrs") and not attrs.get("styles")


# --- Resource limit helpers (parity with nodx_core::parse_str_with_limits) ---


def _first_oversize_line(input_, max_):
    """Return the 1-based line number whose UTF-8 byte length exceeds *max_*."""
    line_no = 1
    cursor = 0
    text = input_
    while True:
        nl = text.find("\n", cursor)
        if nl < 0:
            tail = text[cursor:]
            if len(tail.encode("utf-8")) > max_:
                return line_no
            return None
        chunk = text[cursor:nl]
        if len(chunk.encode("utf-8")) > max_:
            return line_no
        cursor = nl + 1
        line_no += 1


def _front_matter_byte_size(input_):
    if not input_.startswith("---"):
        return None
    if len(input_) > 3 and input_[3] not in ("\n", "\r"):
        return None
    close_idx = input_.find("\n---", 3)
    if close_idx < 0:
        return None
    return len(input_[: close_idx + 4].encode("utf-8"))

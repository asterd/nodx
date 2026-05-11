import re

from .ast import empty_attrs, node
from .attrs import parse_attrs
from .diagnostics import diag
from .front_matter import parse_meta
from .inline_parser import parse_inlines


def parse(input_):
    diagnostics = []
    if input_.startswith("\ufeff"):
        diagnostics.append(diag("NODX-E018", "fatal", "Byte Order Mark is not allowed.", 1, 1))
    if "\u0000" in input_:
        diagnostics.append(diag("NODX-E002", "fatal", "U+0000 is not allowed.", 1, 1))
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
    rows = [table_row(split_pipe(state["lines"][state["pos"]]), True)]
    state["pos"] += 2
    while state["pos"] < len(state["lines"]) and "|" in state["lines"][state["pos"]] and state["lines"][state["pos"]].strip():
        rows.append(table_row(split_pipe(state["lines"][state["pos"]]), False))
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
    if line.startswith("- [ ] ") or line.startswith("- [x] "):
        return "task"
    if line.startswith("- "):
        return "unordered"
    if re.match(r"^\d+\. ", line):
        return "ordered"
    return None


def strip_list(line):
    if line.startswith("- [ ] "):
        return line[6:], False
    if line.startswith("- [x] "):
        return line[6:], True
    if line.startswith("- "):
        return line[2:], None
    return re.sub(r"^\d+\. ", "", line), None


def is_pipe_header(a, b):
    return "|" in a and "-" in b and re.match(r"^[|:\- ]+$", b.strip()) is not None


def split_pipe(line):
    return [part.strip() for part in line.strip().strip("|").split("|")]


def table_row(cells, header):
    children = []
    for cell in cells:
        attrs = empty_attrs()
        if header:
            attrs["attrs"]["header"] = "true"
            attrs["attrs"]["scope"] = "col"
        children.append(node("cell", attrs, [], parse_inlines(cell), None))
    return node("row", empty_attrs(), children, [], None)

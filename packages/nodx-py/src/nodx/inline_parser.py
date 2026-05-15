import re

from .ast import empty_attrs
from .attrs import merge_class_suffix, parse_attrs

# PR2 (RFC §12) extended backslash-escape set: previously
# `` ` * [ ] ( ) { } # @ ~ ^ = : | ``, now adds underscore (disambiguation
# from emphasis), `!`, `.`, `-`, `+`, `<`, `>`, `\`, `"`, `'`. Any other
# character following `\` is preserved as-is together with the backslash.
ESCAPE_CHARS = "`*[](){}#@~^=:|_!.-+<>\\\"'"


def parse_inlines(input_):
    out = []
    i = 0
    while i < len(input_):
        rest = input_[i:]
        # Code span: N backticks open, N backticks close. The opening run
        # length is preserved and the matching close must be exactly the same
        # length, CommonMark-style.
        if rest.startswith("`"):
            run = 0
            while run < len(rest) and rest[run] == "`":
                run += 1
            close_off = _find_backtick_run(rest, run, run)
            if close_off != -1:
                raw = rest[run:close_off]
                out.append({"text": _trim_code_span(raw), "type": "code"})
                i += close_off + run
                continue
            # No matching close: literal backtick. Remaining backticks are
            # re-examined on the next iteration.
            push_text(out, "`")
            i += 1
            continue
        if rest.startswith("$$") and "$$" in rest[2:]:
            end = rest[2:].find("$$") + 2
            out.append({"source": rest[2:end], "type": "math-inline"})
            i += end + 2
        elif rest.startswith("{{") and "}}" in rest:
            end = rest.find("}}")
            raw = rest[2:end]
            parts = raw.split(".", 1)
            if len(parts) == 2:
                out.append({"name": parts[1], "namespace": parts[0], "type": "var"})
            elif raw:
                out.append({"name": raw, "namespace": "vars", "type": "var"})
            else:
                out.append({"text": rest[: end + 2], "type": "text"})
            i += end + 2
        elif rest.startswith("[^") and "]" in rest:
            end = rest.find("]")
            out.append({"target": rest[2:end], "type": "footnote-ref"})
            i += end + 1
        elif rest.startswith("[@") and "]" in rest:
            end = rest.find("]")
            out.append({"target": rest[2:end], "type": "citation-ref"})
            i += end + 1
        elif rest.startswith("@[") and "]" in rest:
            end = rest.find("]")
            out.append({"target": rest[2:end], "type": "ref"})
            i += end + 1
        elif rest.startswith("@{") and "}" in rest:
            end = rest.find("}")
            parts = rest[2:end].split(":", 1)
            if len(parts) == 2:
                out.append({"kind": parts[0], "target": parts[1], "type": "mention"})
            else:
                out.append({"text": rest[: end + 1], "type": "text"})
            i += end + 1
        elif rest.startswith("==") and "==" in rest[2:]:
            end = rest[2:].find("==") + 2
            suffix = parse_span_suffix(rest[end + 2 :])
            item = {"children": parse_inlines(rest[2:end]), "type": "mark"}
            if suffix["consumed"] > 0:
                item["attrs"] = suffix["attrs"]
            out.append(item)
            i += end + 2 + suffix["consumed"]
        elif rest.startswith("~~") and "~~" in rest[2:]:
            end = rest[2:].find("~~") + 2
            out.append({"children": parse_inlines(rest[2:end]), "type": "strike"})
            i += end + 2
        elif rest.startswith("~") and "~" in rest[1:]:
            end = rest[1:].find("~") + 1
            out.append({"children": parse_inlines(rest[1:end]), "type": "sub"})
            i += end + 1
        elif rest.startswith("^") and "^" in rest[1:]:
            end = rest[1:].find("^") + 1
            out.append({"children": parse_inlines(rest[1:end]), "type": "sup"})
            i += end + 1
        elif rest.startswith("**") and "**" in rest[2:]:
            end = rest[2:].find("**") + 2
            out.append({"children": parse_inlines(rest[2:end]), "type": "strong"})
            i += end + 2
        elif rest.startswith("*") and "*" in rest[1:]:
            end = rest[1:].find("*") + 1
            out.append({"children": parse_inlines(rest[1:end]), "type": "em"})
            i += end + 1
        elif rest.startswith("__") or rest.startswith("_"):
            # Underscore emphasis (RFC §12). CommonMark "intraword underscore"
            # rule: a ``_`` run can open emphasis only if it is left-flanking
            # AND (not right-flanking OR preceded by ASCII punctuation), and
            # can close only if it is right-flanking AND (not left-flanking
            # OR followed by ASCII punctuation). Keeps identifiers literal.
            run = 2 if rest.startswith("__") else 1
            opener_preceding = input_[i - 1] if i > 0 else None
            opener_following = input_[i + run] if i + run < len(input_) else None
            if _can_open_underscore(opener_preceding, opener_following):
                closed = _find_underscore_close(rest, run)
                if closed is not None:
                    inner = rest[run : run + closed]
                    out.append({"children": parse_inlines(inner), "type": "strong" if run == 2 else "em"})
                    i += run + closed + run
                    continue
            push_text(out, "_")
            i += 1
        elif rest.startswith("[[") and "]]" in rest:
            close = rest.find("]]")
            label = rest[2:close]
            suffix = parse_span_suffix(rest[close + 2 :])
            if suffix["consumed"] > 0:
                out.append(
                    {
                        "attrs": suffix["attrs"],
                        "children": parse_inlines(label),
                        "type": "span",
                    }
                )
                i += close + 2 + suffix["consumed"]
            else:
                push_text(out, rest[0])
                i += 1
        elif rest.startswith("[") and "]" in rest:
            close = rest.find("]")
            label = rest[1:close]
            after = rest[close + 1 :]
            if after.startswith("(") and ")" in after:
                end = after.find(")")
                after_link = after[end + 1 :]
                attrs = parse_attrs("")
                consumed_attrs = 0
                if after_link.startswith("{") and "}" in after_link:
                    attr_end = after_link.find("}")
                    attrs = parse_attrs(after_link[: attr_end + 1])
                    consumed_attrs = attr_end + 1
                link = {
                    "label": parse_inlines(label),
                    "target": after[1:end],
                    "type": "link",
                }
                if consumed_attrs > 0:
                    link["attrs"] = attrs
                out.append(link)
                i += close + 1 + end + 1 + consumed_attrs
            elif after.startswith("{") and "}" in after:
                suffix = parse_span_suffix(after)
                out.append(
                    {
                        "attrs": suffix["attrs"],
                        "children": parse_inlines(label),
                        "type": "span",
                    }
                )
                i += close + 1 + suffix["consumed"]
            else:
                push_text(out, rest[0])
                i += 1
        elif rest.startswith("<"):
            # Autolink (RFC §12, PR3). Produces an Inline::Link identical to
            # ``[label](target)`` so URL safety stays single-sourced in
            # ``nodx-url`` at validate/render time (no parallel policy here).
            #
            #   <scheme:rest>   -> label = body, target = body
            #   <user@host.tld> -> label = body, target = "mailto:" + body
            auto = _parse_autolink(rest)
            if auto is not None:
                out.append(
                    {
                        "label": [{"text": auto["label"], "type": "text"}],
                        "target": auto["target"],
                        "type": "link",
                    }
                )
                i += auto["consumed"]
                continue
            push_text(out, "<")
            i += 1
        elif rest.startswith("\\") and len(rest) > 1:
            nxt = rest[1]
            # Hard line break: backslash immediately before a newline emits
            # LineBreak and consumes both characters. The paragraph parser
            # joins source lines with `\n`, so this preserves intra-paragraph
            # break semantics across the joined string.
            if nxt == "\n":
                out.append({"type": "line-break"})
                i += 2
                continue
            if nxt in ESCAPE_CHARS:
                push_text(out, nxt)
                i += 2
            else:
                push_text(out, "\\")
                i += 1
        else:
            push_text(out, rest[0])
            i += 1
    return out


def _find_backtick_run(rest, open_len, close_len):
    """Locate a closing backtick run of exactly ``close_len`` ticks."""
    i = open_len
    n = len(rest)
    while i < n:
        if rest[i] == "`":
            start = i
            while i < n and rest[i] == "`":
                i += 1
            if i - start == close_len:
                return start
        else:
            i += 1
    return -1


def _trim_code_span(raw):
    """Normalize a code-span body per the CommonMark trim rule."""
    s = raw.replace("\n", " ")
    if len(s) >= 2 and s.startswith(" ") and s.endswith(" ") and any(c != " " for c in s):
        s = s[1:-1]
    return s


def _find_underscore_close(rest, run):
    """Return the byte offset of a closing underscore run inside ``rest``.

    The offset is relative to ``rest`` skipping the opening run; the run must
    be at least ``run`` underscores long and must satisfy CommonMark's
    "can close underscore" rule.
    """
    j = run
    n = len(rest)
    while j < n:
        if rest[j] != "_":
            j += 1
            continue
        start = j
        while j < n and rest[j] == "_":
            j += 1
        run_len = j - start
        if run_len < run:
            continue
        preceding = rest[start - 1] if start > 0 else None
        following = rest[j] if j < n else None
        if _can_close_underscore(preceding, following):
            return start - run
    return None


def _is_left_flanking(preceding, following):
    if following is None or _is_ascii_whitespace(following):
        return False
    if preceding is None or _is_ascii_whitespace(preceding):
        return True
    if _is_ascii_punct(preceding):
        return True
    return False


def _is_right_flanking(preceding, following):
    if preceding is None or _is_ascii_whitespace(preceding):
        return False
    if following is None or _is_ascii_whitespace(following):
        return True
    if _is_ascii_punct(following):
        return True
    return False


def _can_open_underscore(preceding, following):
    if not _is_left_flanking(preceding, following):
        return False
    if not _is_right_flanking(preceding, following):
        return True
    return preceding is not None and _is_ascii_punct(preceding)


def _can_close_underscore(preceding, following):
    if not _is_right_flanking(preceding, following):
        return False
    if not _is_left_flanking(preceding, following):
        return True
    return following is not None and _is_ascii_punct(following)


def _is_ascii_alnum(ch):
    return ch.isascii() and ch.isalnum()


def _is_ascii_whitespace(ch):
    return ch in (" ", "\t", "\n", "\r")


def _is_ascii_punct(ch):
    if not ch or ord(ch) > 0x7F:
        return False
    return not _is_ascii_alnum(ch) and not _is_ascii_whitespace(ch)


def parse_span_suffix(input_):
    attrs = empty_attrs()
    consumed = 0
    saw = False
    while consumed < len(input_):
        rest = input_[consumed:]
        if rest.startswith("{") and "}" in rest:
            end = rest.find("}")
            parsed = parse_attrs(rest[: end + 1])
            attrs["id"] = parsed.get("id") or attrs["id"]
            attrs["classes"].extend(parsed["classes"])
            attrs["attrs"].update(parsed["attrs"])
            attrs["styles"].update(parsed.get("styles", {}))
            consumed += end + 1
            saw = True
        elif rest.startswith("."):
            match = re.match(r"^(\.[A-Za-z_][A-Za-z0-9_-]*)+", rest)
            if not match:
                break
            merge_class_suffix(attrs, match.group(0))
            consumed += len(match.group(0))
            saw = True
        else:
            break
    attrs["classes"] = sorted(set(attrs["classes"]))
    if not attrs["styles"]:
        del attrs["styles"]
    return {"attrs": attrs, "consumed": consumed if saw else 0}


# Autolink grammar (RFC §12, PR3). Mirrors ``parse_autolink`` in
# ``crates/nodx-core/src/inline_parser.rs`` byte-for-byte to keep the AST
# triplet identical. Returns ``{"consumed", "label", "target"}`` or ``None``.
def _parse_autolink(rest):
    # ``rest[0]`` is ``'<'`` by precondition.
    end = 1
    while end < len(rest):
        ch = rest[end]
        if ch == ">":
            break
        code = ord(ch)
        if ch == "<" or ch in ("\n", "\r", "\t", " ") or code < 0x20 or code == 0x7F:
            return None
        end += 1
    if end >= len(rest) or rest[end] != ">":
        return None
    body = rest[1:end]
    if not body:
        return None

    # 1) Absolute URI autolink
    colon = body.find(":")
    if colon > 0 and _is_valid_autolink_scheme(body[:colon]):
        return {"consumed": end + 1, "label": body, "target": body}

    # 2) Email autolink
    if _is_valid_autolink_email(body):
        return {"consumed": end + 1, "label": body, "target": f"mailto:{body}"}

    return None


def _is_valid_autolink_scheme(scheme):
    if not (2 <= len(scheme) <= 32):
        return False
    if not scheme[0].isascii() or not scheme[0].isalpha():
        return False
    for ch in scheme[1:]:
        if ch.isascii() and (ch.isalnum() or ch in "+.-"):
            continue
        return False
    return True


def _is_valid_autolink_email(body):
    if "@" not in body:
        return False
    local, _, domain = body.partition("@")
    if not local or not domain:
        return False
    for ch in local:
        if not (ch.isascii() and (ch.isalnum() or ch in "._%+-")):
            return False
    labels = domain.split(".")
    if len(labels) < 2:
        return False
    for label in labels:
        if not label:
            return False
        for ch in label:
            if not (ch.isascii() and (ch.isalnum() or ch == "-")):
                return False
    tld = labels[-1]
    if len(tld) < 2:
        return False
    for ch in tld:
        if not (ch.isascii() and ch.isalpha()):
            return False
    return True


def push_text(out, text):
    if out and out[-1]["type"] == "text":
        out[-1]["text"] += text
    else:
        out.append({"text": text, "type": "text"})

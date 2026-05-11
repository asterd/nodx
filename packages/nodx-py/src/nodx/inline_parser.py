from .attrs import parse_attrs


def parse_inlines(input_):
    out = []
    i = 0
    while i < len(input_):
        rest = input_[i:]
        if rest.startswith("`") and "`" in rest[1:]:
            end = rest[1:].find("`") + 1
            out.append({"text": rest[1:end], "type": "code"})
            i += end + 1
        elif rest.startswith("$$") and "$$" in rest[2:]:
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
            out.append({"children": parse_inlines(rest[2:end]), "type": "mark"})
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
                end = after.find("}")
                out.append(
                    {
                        "attrs": parse_attrs(after[: end + 1]),
                        "children": parse_inlines(label),
                        "type": "span",
                    }
                )
                i += close + 1 + end + 1
            else:
                push_text(out, rest[0])
                i += 1
        elif (
            rest.startswith("\\")
            and len(rest) > 1
            and rest[1] in "`*[](){}#@~^=:|"
        ):
            push_text(out, rest[1])
            i += 2
        else:
            push_text(out, rest[0])
            i += 1
    return out


def push_text(out, text):
    if out and out[-1]["type"] == "text":
        out[-1]["text"] += text
    else:
        out.append({"text": text, "type": "text"})

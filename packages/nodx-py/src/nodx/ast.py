def empty_attrs():
    return {"attrs": {}, "classes": [], "id": None}


def node(type_, attrs, children, inlines, text):
    return {
        "attrs": attrs["attrs"],
        "children": children,
        "classes": attrs["classes"],
        "id": attrs["id"],
        "inlines": inlines,
        "text": text,
        "type": type_,
    }


def first_heading(nodes):
    for item in nodes:
        if item["type"] == "heading":
            return plain_node_text(item)
        child = first_heading(item["children"])
        if child:
            return child
    return None


def plain_node_text(item):
    parts = [plain_inlines(item["inlines"])]
    for child in item["children"]:
        text = plain_node_text(child)
        if text:
            parts.append(text)
    return " ".join(part for part in parts if part)


def plain_inlines(inlines):
    out = ""
    for item in inlines:
        type_ = item["type"]
        if type_ in ("text", "code"):
            out += item["text"]
        elif type_ == "math-inline":
            out += item["source"]
        elif type_ in ("strong", "em", "mark", "sub", "sup"):
            out += plain_inlines(item["children"])
        elif type_ == "link":
            out += plain_inlines(item["label"])
        elif type_ == "span":
            out += plain_inlines(item["children"])
        elif type_ == "var":
            out += "{{" + item["namespace"] + "." + item["name"] + "}}"
        elif type_ in ("ref", "footnote-ref", "citation-ref"):
            out += item["target"]
        elif type_ == "mention":
            out += "@" + item["kind"] + ":" + item["target"]
    return out

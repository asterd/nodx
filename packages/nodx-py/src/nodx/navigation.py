from .ast import plain_inlines, plain_node_text


def resolve_navigation(doc):
    ids = {}
    collect_id_paths(doc["body"], "", ids)
    navigations = []
    collect_tocs(doc["body"], "", doc, ids, navigations)
    return {"navigations": navigations}


def collect_id_paths(nodes, prefix, ids):
    for i, item in enumerate(nodes):
        path = child_path(prefix, i)
        if item["id"] is not None:
            ids[item["id"]] = path
        collect_id_paths(item["children"], path, ids)


def collect_tocs(nodes, prefix, doc, ids, out):
    for i, item in enumerate(nodes):
        path = child_path(prefix, i)
        if item["type"] == "toc":
            out.append(resolve_toc(item, path, doc, ids))
        collect_tocs(item["children"], path, doc, ids, out)


def resolve_toc(toc, path, doc, ids):
    role = toc["attrs"].get("role", "primary")
    label = toc["attrs"].get("title", default_navigation_label(role))
    scope = toc["attrs"].get("scope")
    if toc["attrs"].get("mode") == "manual":
        entries = []
        collect_manual_entries(toc["children"], ids, entries)
        return {"entries": entries, "label": label, "role": role, "scope": scope, "tocId": toc["id"], "tocPath": path}
    source_path = ids.get(scope[1:]) if isinstance(scope, str) and scope.startswith("#") else None
    scoped_node = node_at_path(doc["body"], source_path) if source_path else None
    source_nodes = [scoped_node] if scoped_node else doc["body"]
    min_level = parse_level(toc["attrs"].get("min-level"))
    max_level = parse_level(toc["attrs"].get("max-level"))
    depth = parse_level(toc["attrs"].get("depth")) or 6
    base_level = first_heading_level(source_nodes) or 1
    effective_min = min_level or base_level
    effective_max = max_level or min(base_level + depth - 1, 6)
    entries = []
    collect_entries(source_nodes, "", effective_min, effective_max, entries)
    return {"entries": entries, "label": label, "role": role, "scope": scope, "tocId": toc["id"], "tocPath": path}


def collect_entries(nodes, prefix, min_level, max_level, out):
    for i, item in enumerate(nodes):
        path = child_path(prefix, i)
        if item["type"] == "heading" and item["id"] is not None:
            level = heading_level(item)
            if level is not None and min_level <= level <= max_level and item["attrs"].get("toc-hidden") != "true":
                out.append({
                    "id": item["id"],
                    "level": parse_level(item["attrs"].get("toc-level")) or level,
                    "path": path,
                    "title": item["attrs"].get("toc", plain_node_text(item)),
                })
        collect_entries(item["children"], path, min_level, max_level, out)


def collect_manual_entries(nodes, ids, out):
    for item in nodes:
        collect_manual_inline_entries(item["inlines"], ids, out)
        collect_manual_entries(item["children"], ids, out)


def collect_manual_inline_entries(inlines, ids, out):
    for item in inlines:
        if item["type"] == "link":
            if item["target"].startswith("#"):
                id_ = item["target"][1:]
                out.append({"id": id_, "level": 1, "path": ids.get(id_, ""), "title": plain_inlines(item["label"])})
            collect_manual_inline_entries(item["label"], ids, out)
        elif item["type"] in ("strong", "em", "mark", "sub", "sup", "span"):
            collect_manual_inline_entries(item["children"], ids, out)


def first_heading_level(nodes):
    for item in nodes:
        if item["type"] == "heading":
            level = heading_level(item)
            if level is not None:
                return level
        level = first_heading_level(item["children"])
        if level is not None:
            return level
    return None


def node_at_path(nodes, path):
    current = nodes
    item = None
    for part in path.split("."):
        index = int(part)
        if index >= len(current):
            return None
        item = current[index]
        current = item["children"]
    return item


def heading_level(item):
    return parse_level(item["attrs"].get("level"))


def parse_level(value):
    if not isinstance(value, str) or not value.isdigit():
        return None
    level = int(value)
    return level if 1 <= level <= 6 else None


def default_navigation_label(role):
    if role == "local":
        return "In this section"
    if role == "secondary":
        return "Secondary navigation"
    if role == "breadcrumb":
        return "Breadcrumb"
    return "Table of contents"


def child_path(prefix, index):
    return str(index) if prefix == "" else prefix + "." + str(index)

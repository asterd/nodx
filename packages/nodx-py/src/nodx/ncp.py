import json

from .ast import plain_inlines
from .bytes import sha256_base64_url
from .canonical import canonical_json, canonical_stringify
from .navigation import resolve_navigation


def ncp_json(doc):
    canonical = canonical_json(doc)
    ids = collect_node_ids(doc["body"], "")
    navigation = resolve_navigation(doc)
    return (
        '{"chunks":[{"id":"chunk-1","nodes":'
        + json.dumps(ids, ensure_ascii=False, separators=(",", ":"))
        + ',"sha256":'
        + json.dumps(sha256_base64_url("\n".join(ids)))
        + '}],"loss":[],"mode":"semantic","nodes":'
        + ncp_nodes_json(doc["body"], "", navigation)
        + ',"schema":"nodx-ncp/1.0","sourceHash":'
        + json.dumps(sha256_base64_url(canonical))
        + "}"
    )


def collect_node_ids(nodes, prefix):
    out = []
    for i, item in enumerate(nodes):
        path = child_path(prefix, i)
        out.append(item["id"] if item["id"] is not None else "path:" + path)
        out.extend(collect_node_ids(item["children"], path))
    return out


def ncp_nodes_json(nodes, prefix, navigation):
    items = []
    for i, item in enumerate(nodes):
        path = child_path(prefix, i)
        out = (
            '{"attrs":'
            + canonical_stringify(item["attrs"])
            + ',"children":'
            + ncp_nodes_json(item["children"], path, navigation)
            + ',"classes":'
            + canonical_stringify(item.get("classes") or [])
            + ',"id":'
            + json.dumps(item["id"] or "", ensure_ascii=False)
            + ',"path":'
            + json.dumps(path)
            + ',"sha256":'
            + json.dumps(sha256_base64_url(ncp_node_hash_input(item)))
            + ',"text":'
            + json.dumps(item["text"] if item["text"] is not None else plain_inlines(item["inlines"]), ensure_ascii=False)
            + ',"type":'
            + json.dumps(item["type"])
        )
        if item["type"] == "toc":
            nav = next((candidate for candidate in navigation["navigations"] if candidate["tocPath"] == path), None)
            out += ',"navigationEntries":' + navigation_entries_json(nav["entries"] if nav else [])
        items.append(out + "}")
    return "[" + ",".join(items) + "]"


def navigation_entries_json(entries):
    return "[" + ",".join(
        '{"id":'
        + json.dumps(entry["id"], ensure_ascii=False)
        + ',"level":'
        + str(entry["level"])
        + ',"path":'
        + json.dumps(entry["path"])
        + ',"title":'
        + json.dumps(entry["title"], ensure_ascii=False)
        + "}"
        for entry in entries
    ) + "]"


def ncp_node_hash_input(item):
    out = item["type"] + "\n" + (item["id"] or "") + "\n" + canonical_stringify(item.get("classes") or []) + "\n" + canonical_stringify(item["attrs"]) + "\n"
    if item.get("styles"):
        out += canonical_stringify(item["styles"]) + "\n"
    out += item["text"] or ""
    out += plain_inlines(item["inlines"])
    for child in item["children"]:
        out += "\n" + ncp_node_hash_input(child)
    return out


def child_path(prefix, index):
    return str(index) if prefix == "" else prefix + "." + str(index)

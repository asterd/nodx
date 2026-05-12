import { plainInlines } from "./ast.mjs";
import { sha256Base64Url } from "./bytes.mjs";
import { canonicalJson, canonicalStringify } from "./canonical.mjs";
import { resolveNavigation } from "./navigation.mjs";

export function ncpJson(doc) {
  const canonical = canonicalJson(doc);
  const ids = collectNodeIds(doc.body, "");
  const navigation = resolveNavigation(doc);
  return "{\"chunks\":[{\"id\":\"chunk-1\",\"nodes\":" + JSON.stringify(ids) +
    ",\"sha256\":" + JSON.stringify(sha256Base64Url(ids.join("\n"))) +
    "}],\"loss\":[],\"mode\":\"semantic\",\"nodes\":" + ncpNodesJson(doc.body, "", navigation) +
    ",\"schema\":\"nodx-ncp/1.0\",\"sourceHash\":" + JSON.stringify(sha256Base64Url(canonical)) + "}";
}

function collectNodeIds(nodes, prefix) {
  const out = [];
  nodes.forEach((item, i) => {
    const path = childPath(prefix, i);
    out.push(item.id ?? "path:" + path);
    out.push(...collectNodeIds(item.children, path));
  });
  return out;
}

function ncpNodesJson(nodes, prefix, navigation) {
  return "[" + nodes.map((item, i) => {
    const path = childPath(prefix, i);
    let out = "{\"attrs\":" + canonicalStringify(item.attrs) +
      ",\"children\":" + ncpNodesJson(item.children, path, navigation) +
      ",\"classes\":" + canonicalStringify(item.classes ?? []) +
      ",\"id\":" + JSON.stringify(item.id ?? "") +
      ",\"path\":" + JSON.stringify(path) +
      ",\"sha256\":" + JSON.stringify(sha256Base64Url(ncpNodeHashInput(item))) +
      ",\"text\":" + JSON.stringify(item.text ?? plainInlines(item.inlines)) +
      ",\"type\":" + JSON.stringify(item.type);
    if (item.type === "toc") {
      const nav = navigation.navigations.find((candidate) => candidate.tocPath === path);
      out += ",\"navigationEntries\":" + navigationEntriesJson(nav?.entries ?? []);
    }
    return out + "}";
  }).join(",") + "]";
}

function navigationEntriesJson(entries) {
  return "[" + entries.map((entry) => "{\"id\":" + JSON.stringify(entry.id) +
    ",\"level\":" + entry.level +
    ",\"path\":" + JSON.stringify(entry.path) +
    ",\"title\":" + JSON.stringify(entry.title) + "}").join(",") + "]";
}

function ncpNodeHashInput(item) {
  let out = item.type + "\n" + (item.id ?? "") + "\n" + canonicalStringify(item.classes ?? []) + "\n" + canonicalStringify(item.attrs) + "\n";
  if (item.styles && Object.keys(item.styles).length) out += canonicalStringify(item.styles) + "\n";
  out += item.text ?? "";
  out += plainInlines(item.inlines);
  for (const child of item.children) out += "\n" + ncpNodeHashInput(child);
  return out;
}

function childPath(prefix, index) {
  return prefix === "" ? String(index) : prefix + "." + index;
}

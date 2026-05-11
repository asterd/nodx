import { plainNodeText } from "./ast.mjs";

export function resolveNavigation(doc) {
  const ids = new Map();
  collectIdPaths(doc.body, "", ids);
  const navigations = [];
  collectTocs(doc.body, "", doc, ids, navigations);
  return { navigations };
}

function collectIdPaths(nodes, prefix, ids) {
  nodes.forEach((item, i) => {
    const path = childPath(prefix, i);
    if (item.id !== null) ids.set(item.id, path);
    collectIdPaths(item.children, path, ids);
  });
}

function collectTocs(nodes, prefix, doc, ids, out) {
  nodes.forEach((item, i) => {
    const path = childPath(prefix, i);
    if (item.type === "toc") out.push(resolveToc(item, path, doc, ids));
    collectTocs(item.children, path, doc, ids, out);
  });
}

function resolveToc(toc, path, doc, ids) {
  const role = toc.attrs.role ?? "primary";
  const label = toc.attrs.title ?? defaultNavigationLabel(role);
  const scope = toc.attrs.scope ?? null;
  const sourcePath = scope?.startsWith("#") ? ids.get(scope.slice(1)) : null;
  const scopedNode = sourcePath ? nodeAtPath(doc.body, sourcePath) : null;
  const sourceNodes = scopedNode ? [scopedNode] : doc.body;
  const minLevel = parseLevel(toc.attrs["min-level"]);
  const maxLevel = parseLevel(toc.attrs["max-level"]);
  const depth = parseLevel(toc.attrs.depth) ?? 6;
  const baseLevel = firstHeadingLevel(sourceNodes) ?? 1;
  const effectiveMin = minLevel ?? baseLevel;
  const effectiveMax = maxLevel ?? Math.min(baseLevel + depth - 1, 6);
  const entries = [];
  collectEntries(sourceNodes, "", effectiveMin, effectiveMax, entries);
  return { entries, label, role, scope, tocId: toc.id, tocPath: path };
}

function collectEntries(nodes, prefix, minLevel, maxLevel, out) {
  nodes.forEach((item, i) => {
    const path = childPath(prefix, i);
    if (item.type === "heading" && item.id !== null) {
      const level = headingLevel(item);
      if (level !== null && level >= minLevel && level <= maxLevel) {
        out.push({ id: item.id, level, path, title: plainNodeText(item) });
      }
    }
    collectEntries(item.children, path, minLevel, maxLevel, out);
  });
}

function firstHeadingLevel(nodes) {
  for (const item of nodes) {
    if (item.type === "heading") {
      const level = headingLevel(item);
      if (level !== null) return level;
    }
    const level = firstHeadingLevel(item.children);
    if (level !== null) return level;
  }
  return null;
}

function nodeAtPath(nodes, path) {
  let current = nodes;
  let item = null;
  for (const part of path.split(".")) {
    const index = Number.parseInt(part, 10);
    item = current[index];
    if (!item) return null;
    current = item.children;
  }
  return item;
}

function headingLevel(item) {
  return parseLevel(item.attrs.level);
}

function parseLevel(value) {
  if (typeof value !== "string" || !/^[0-9]+$/.test(value)) return null;
  const level = Number.parseInt(value, 10);
  return level >= 1 && level <= 6 ? level : null;
}

function defaultNavigationLabel(role) {
  if (role === "local") return "In this section";
  if (role === "secondary") return "Secondary navigation";
  if (role === "breadcrumb") return "Breadcrumb";
  return "Table of contents";
}

function childPath(prefix, index) {
  return prefix === "" ? String(index) : prefix + "." + index;
}

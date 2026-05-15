import { parse } from "./blockParser.mjs";
import { resolveNavigation } from "./navigation.mjs";
import { classifyUri, ReferenceKind } from "./url.mjs";
import { sanitizeStylesheet, yamlStyleToCss } from "./nods.mjs";

export const THEME_NAMES = ["none", "plain", "base", "web", "print", "presentation", "docs"];

export function renderFragment(doc, options = {}) {
  const navigation = resolveNavigation(doc);
  const renderOptions = { ...options, remoteAssets: documentAllowsRemoteAssets(doc) };
  const stylesheets = [...(Array.isArray(doc.meta.stylesheets) ? doc.meta.stylesheets : []), ...(options.stylesheets ?? [])];
  const extraStyles = stylesheets.map((css) => `<style>${sanitizeStylesheet(css)}</style>`).join("");
  const componentStyles = componentDefinitions(doc).map((def) => def.style ? `<style>${sanitizeStylesheet(def.style)}</style>` : "").join("");
  return extraStyles + componentStyles + doc.body.map((node, index) => renderNode(node, String(index), navigation, { ...renderOptions, doc })).join("");
}

export function renderHtml(doc, options = {}) {
  const title = typeof doc.meta.title === "string" ? doc.meta.title : deriveTitle(doc.body);
  const lang = typeof doc.meta.language === "string" && doc.meta.language !== "und" ? ` lang="${escapeAttr(doc.meta.language)}"` : "";
  const dir = typeof doc.meta.dir === "string" && doc.meta.dir !== "auto" ? ` dir="${escapeAttr(doc.meta.dir)}"` : "";
  const body = isDocsLayout(doc) ? renderDocsBody(doc, options) : `<body>${renderFragment(doc, options)}</body>`;
  const pageCss = pageStylesheet(doc);
  return `<!doctype html><html${lang}${dir}><meta charset="utf-8"><style>${themeStylesheet(doc.meta.theme)}</style>${pageCss ? `<style>${pageCss}</style>` : ""}${title ? `<title>${escapeHtml(title)}</title>` : ""}${body}</html>`;
}

export function themeStylesheet(theme = "base") {
  const name = THEME_NAMES.includes(theme) ? theme : "base";
  if (name === "none") return "html[dir=\"rtl\"]{direction:rtl}";
  if (name === "plain") return "html[dir=\"rtl\"]{direction:rtl}body{font:16px/1.55 system-ui,-apple-system,BlinkMacSystemFont,\"Segoe UI\",sans-serif;color:#1f2937;background:#ffffff}";
  const tokens = standardTokens();
  const common = commonStyles();
  if (name === "print") {
    return tokens + ":root{--nodx-font-body:Georgia,\"Times New Roman\",serif;--nodx-font-heading:var(--nodx-font-body);--nodx-color-heading:#111827;--nodx-color-primary:#374151;--nodx-color-accent:#7f1d1d;--nodx-color-surface:#ffffff}body{font:11pt/1.55 var(--nodx-font-body);max-width:none;margin:0;color:var(--nodx-color-text);background:var(--nodx-color-bg)}@page{size:A4;margin:var(--nodx-page-margin)}h1,h2,h3{break-after:avoid}table,figure,aside,.nodx-callout{break-inside:avoid}.pagebreak{break-before:page;border:0;margin:0}" + common;
  }
  if (name === "presentation") {
    return tokens + ":root{--nodx-color-bg:#f8f7ff;--nodx-color-heading:#312e81;--nodx-color-primary:#7c3aed;--nodx-color-accent:#e11d48;--nodx-color-rule:#ddd6fe;--nodx-color-surface:#ffffff}body{font:28px/1.45 var(--nodx-font-body);max-width:1100px;margin:40px auto;padding:0 28px;color:var(--nodx-color-text);background:var(--nodx-color-bg)}h1{font-size:2.4em}h2{font-size:1.8em}" + common;
  }
  if (name === "web") {
    return tokens + ":root{--nodx-color-bg:#f8fafc;--nodx-color-heading:#0f172a;--nodx-color-primary:#2563eb;--nodx-color-accent:#be123c;--nodx-color-rule:#cbd5e1;--nodx-color-surface:#ffffff}body{font:16px/1.65 var(--nodx-font-body);max-width:960px;margin:32px auto;padding:0 18px;color:var(--nodx-color-text);background:var(--nodx-color-bg)}" + common;
  }
  if (name === "docs") {
    return tokens + ":root{--nodx-color-bg:#ffffff;--nodx-color-heading:#172554;--nodx-color-primary:#1d4ed8;--nodx-color-accent:#7c3aed;--nodx-color-rule:#dbe3ef;--nodx-color-surface:#f8fafc}" + docsStyles() + common;
  }
  return tokens + ":root{--nodx-color-heading:#111827;--nodx-color-surface:#f9fafb}body{font:16px/1.6 var(--nodx-font-body);max-width:920px;margin:32px auto;padding:0 16px;color:var(--nodx-color-text);background:var(--nodx-color-bg)}" + common;
}

function renderDocsBody(doc, options) {
  const navigation = resolveNavigation(doc);
  const entries = navigation.navigations[0]?.entries ?? collectHeadingEntries(doc.body);
  return `<body class="nodx-docs-layout"><aside class="nodx-docs-sidebar"><a class="nodx-docs-brand" href="#">${escapeHtml(docsTitle(doc))}</a>${docsNav(entries.filter((entry) => entry.level === 1), 1)}</aside><main class="nodx-docs-main">${renderFragment(doc, options)}</main><aside class="nodx-docs-outline">${docsNav(entries.filter((entry) => entry.level > 1), 6)}</aside></body>`;
}

function docsNav(entries, maxLevel) {
  const items = entries.filter((entry) => entry.id && entry.level <= maxLevel).map((entry) => `<li data-level="${escapeAttr(String(entry.level))}"><a href="#${escapeAttr(entry.id)}">${escapeHtml(entry.title)}</a></li>`).join("");
  return items ? `<nav><ol>${items}</ol></nav>` : "";
}

function collectHeadingEntries(nodes, path = "", out = []) {
  nodes.forEach((node, index) => {
    const nextPath = childPath(path, index);
    if (node.type === "heading") {
      out.push({ id: node.id ?? "", level: Number(node.attrs.level ?? 1), path: nextPath, title: plainInlines(node.inlines) });
    }
    collectHeadingEntries(node.children ?? [], nextPath, out);
  });
  return out;
}

function docsTitle(doc) {
  return typeof doc.meta.title === "string" ? doc.meta.title : deriveTitle(doc.body) ?? "Documentation";
}

function isDocsLayout(doc) {
  return doc.meta.layout === "docs" || doc.meta.theme === "docs";
}

export function renderSemanticText(doc) {
  const lines = [];
  writeSemanticNodes(doc.body, lines, []);
  return lines.join("\n").replace(/\n{3,}/g, "\n\n").trim() + "\n";
}

function writeSemanticNodes(nodes, lines, path) {
  let index = 0;
  for (const node of nodes) {
    if (["style", "pagebreak", "toc", "hr"].includes(node.type)) continue;
    const nextPath = [...path, index];
    index += 1;
    writeSemanticNode(node, lines, nextPath);
  }
}

function writeSemanticNode(node, lines, path) {
  if (node.type === "heading") {
    lines.push(`${"#".repeat(clamp(Number(node.attrs.level ?? 1), 1, 6))} ${plainInlines(node.inlines)}${semanticId(node)}`);
    return;
  }
  if (node.type === "paragraph") {
    pushTextLine(lines, plainInlines(node.inlines));
    return;
  }
  if (node.type === "list") {
    const ordered = node.attrs.kind === "ordered";
    let i = 1;
    for (const item of node.children.filter((child) => child.type === "item")) {
      const marker = ordered ? `${i}.` : item.attrs.checked === "true" ? "- [x]" : item.attrs.checked === "false" ? "- [ ]" : "-";
      lines.push(`${marker} ${semanticNodeText(item)}`.trim());
      i += 1;
    }
    return;
  }
  if (node.type === "table") {
    writeSemanticTable(node, lines);
    return;
  }
  if (node.type === "figure") {
    lines.push(`Figure${semanticId(node)}:`);
    writeSemanticNodes(node.children ?? [], lines, path);
    return;
  }
  if (node.type === "image") {
    lines.push(`Image${semanticId(node)}: alt="${node.attrs.alt ?? ""}" src="${node.attrs.src ?? ""}"`);
    return;
  }
  if (node.type === "caption") {
    pushTextLine(lines, `Caption: ${semanticNodeText(node)}`);
    return;
  }
  if (node.type === "code" || node.type === "pre" || node.type === "math") {
    const lang = node.attrs.lang ? ` ${node.attrs.lang}` : "";
    lines.push(`\`\`\`${node.type}${lang}`);
    lines.push(node.text ?? "");
    lines.push("```");
    return;
  }
  if (node.type === "quote") {
    for (const line of semanticNodeText(node).split(/\r?\n/)) lines.push(`> ${line}`);
    return;
  }
  if (isCalloutNode(node)) {
    lines.push(`${calloutLabel(calloutType(node))}${semanticAttrs(node)}: ${semanticNodeText(node)}`.trim());
    return;
  }
  if (node.type === "form") {
    lines.push(`Form${semanticId(node)}:`);
    for (const field of node.children.filter((child) => child.type === "field")) {
      lines.push(`- ${field.attrs.label ?? field.attrs.name ?? "Field"}: ${field.attrs.value ?? semanticNodeText(field)}`);
    }
    return;
  }
  if (node.type === "field") {
    lines.push(`${fieldLabel(node)}: ${node.attrs.value ?? semanticNodeText(node)}`);
    return;
  }
  if (node.type === "media" || node.type === "embed" || node.type === "include") {
    lines.push(`${capitalize(node.type)}${semanticAttrs(node)}: ${semanticNodeText(node)}`.trim());
    return;
  }
  if (node.type === "bibliography") {
    lines.push("Bibliography:");
    writeSemanticNodes(node.children ?? [], lines, path);
    return;
  }
  if (node.type === "citation-entry") {
    lines.push(`- ${semanticNodeText(node)}`);
    return;
  }
  if (node.type.includes("-")) {
    lines.push(`Component ${node.type}${semanticAttrs(node)}:`);
    writeSemanticNodes(node.children ?? [], lines, path);
    return;
  }
  const text = semanticNodeText(node);
  if (text) lines.push(`${capitalize(node.type)}${semanticAttrs(node)}: ${text}`);
  else writeSemanticNodes(node.children ?? [], lines, path);
}

function writeSemanticTable(node, lines) {
  const rows = node.children.filter((child) => child.type === "row").map((row) => row.children.filter((cell) => cell.type === "cell"));
  if (!rows.length) {
    lines.push(`Table${semanticId(node)}: empty`);
    return;
  }
  lines.push(`Table${semanticId(node)}:`);
  const values = rows.map((row) => row.map((cell) => escapeMarkdownCell(semanticNodeText(cell))));
  const width = Math.max(...values.map((row) => row.length));
  const normalized = values.map((row) => [...row, ...Array(Math.max(0, width - row.length)).fill("")]);
  const firstRowIsHeader = rows[0].every((cell) => cell.attrs.header === "true");
  const header = firstRowIsHeader ? normalized[0] : Array.from({ length: width }, (_, index) => `Column ${index + 1}`);
  lines.push(`| ${header.join(" | ")} |`);
  lines.push(`| ${Array(width).fill("---").join(" | ")} |`);
  for (const row of normalized.slice(firstRowIsHeader ? 1 : 0)) lines.push(`| ${row.join(" | ")} |`);
}

function semanticNodeText(node) {
  const parts = [];
  const inline = plainInlines(node.inlines ?? []).trim();
  if (inline) parts.push(inline);
  if (node.text !== null && node.text !== undefined && node.text.trim()) parts.push(node.text.trim());
  for (const child of node.children ?? []) {
    if (["style", "pagebreak", "toc", "hr"].includes(child.type)) continue;
    if (child.type === "row" || child.type === "cell") {
      const text = semanticNodeText(child);
      if (text) parts.push(text);
    } else if (child.type === "paragraph" || child.type === "caption" || child.type === "item") {
      const text = semanticNodeText(child);
      if (text) parts.push(text);
    }
  }
  return parts.join(" ").replace(/\s+/g, " ").trim();
}

function semanticId(node) {
  return node.id ? ` #${node.id}` : "";
}

function semanticAttrs(node) {
  const attrs = Object.entries(node.attrs ?? {})
    .filter(([key, value]) => value !== "" && value !== null && value !== undefined && !["level", "header", "scope"].includes(key))
    .sort(([left], [right]) => left.localeCompare(right))
    .map(([key, value]) => `${key}="${String(value)}"`);
  const id = node.id ? [`id="${node.id}"`] : [];
  const all = [...id, ...attrs];
  return all.length ? ` [${all.join(" ")}]` : "";
}

function fieldLabel(node) {
  return node.attrs.label ?? node.attrs.name ?? "Field";
}

function pushTextLine(lines, text) {
  const normalized = text.trim();
  if (normalized) lines.push(normalized);
}

function escapeMarkdownCell(text) {
  return text.replace(/\|/g, "\\|").replace(/\s+/g, " ").trim();
}

function capitalize(text) {
  return text ? text[0].toUpperCase() + text.slice(1) : text;
}

function renderNode(node, path, navigation, options) {
  const custom = options.componentRenderers?.[node.type];
  if (custom) {
    return custom({
      node,
      path,
      renderChildren: () => renderChildren(node, path, navigation, options),
      renderInlines: (inlines) => renderInlines(inlines, options),
      escapeHtml,
      attrs: htmlAttrs(node),
    });
  }
  const component = componentDefinitions(options.doc).find((def) => def.name === node.type && typeof def.template === "string");
  if (component) return renderComponentTemplate(component, node, path, navigation, options);
  switch (node.type) {
    case "heading": {
      const level = clamp(Number(node.attrs.level ?? 1), 1, 6);
      return `<h${level}${htmlAttrs(node)}>${renderInlines(node.inlines, options)}</h${level}>`;
    }
    case "paragraph": return wrapInlines("p", node, path, navigation, options);
    case "section": return wrapChildren("section", node, path, navigation, options);
    case "note": return renderCallout(node, path, navigation, options);
    case "info":
    case "tip":
    case "important":
    case "caution":
    case "warning":
    case "danger":
    case "example":
    case "summary": return renderCallout(node, path, navigation, options);
    case "quote": return wrapChildren("blockquote", node, path, navigation, options);
    case "list": return wrapChildren(node.attrs.kind === "ordered" ? "ol" : "ul", node, path, navigation, options);
    case "item": return wrapInlines("li", node, path, navigation, options);
    case "code":
    case "pre": return `<pre><code${node.attrs.lang ? ` data-lang="${escapeAttr(node.attrs.lang)}"` : ""}>${escapeHtml(node.text ?? "")}</code></pre>`;
    case "math": return `<pre class="math">${escapeHtml(node.text ?? "")}</pre>`;
    case "style": return `<style>${renderStyle(node)}</style>`;
    case "table": return renderTable(node, path, navigation, options);
    case "row": return wrapChildren("tr", node, path, navigation, options);
    case "cell": return wrapInlines(node.attrs.header === "true" ? "th" : "td", node, path, navigation, options);
    case "figure": return wrapChildren("figure", node, path, navigation, options);
    case "caption": return wrapInlines("figcaption", node, path, navigation, options);
    case "image": return renderImage(node, options);
    case "form": return wrapChildren("dl", node, path, navigation, options);
    case "field": return `<div${htmlId(node)}><dt>${escapeHtml(node.attrs.label ?? node.attrs.name ?? "Field")}</dt><dd>${escapeHtml(node.attrs.value ?? "")}</dd></div>`;
    case "toc": return renderToc(node, path, navigation);
    case "pagebreak": return `<hr${htmlId(node)} class="pagebreak">`;
    // Thematic break: void element rendered with whatever attrs (none today)
    // the AST carries. Mirrors `nodx-render-html::render_node` "hr" arm.
    case "hr": return `<hr${htmlAttrs(node)}>`;
    case "media":
    case "embed":
    case "include": return renderMediaFallback(node, path, navigation, options);
    case "bibliography": return wrapChildren("ol", node, path, navigation, options);
    case "citation-entry": return wrapInlines("li", node, path, navigation, options);
    case "speaker-notes": return `<aside${htmlAttrsWithoutClass(node)} class="${classAttr(node, "speaker-notes")}" aria-label="Speaker notes">${renderInlines(node.inlines, options)}${renderChildren(node, path, navigation, options)}</aside>`;
    case "grid": return wrapLayoutChildren("nodx-grid", node, path, navigation, options);
    case "columns": return wrapLayoutChildren("nodx-columns", node, path, navigation, options);
    case "frame": return wrapLayoutChildren("nodx-frame", node, path, navigation, options);
    case "page": return wrapLayoutChildren("nodx-page", node, path, navigation, options);
    default:
      if (node.type.includes("-")) {
        return `<section${htmlAttrsWithoutClass(node)} class="${classAttr(node, "nodx-component nodx-component--fallback")}" data-component="${escapeAttr(node.type)}"><p class="nodx-component__title">${escapeHtml(node.type)} fallback</p>${renderInlines(node.inlines, options)}${renderChildren(node, path, navigation, options)}</section>`;
      }
      return wrapChildren("div", node, path, navigation, options);
  }
}

function renderTable(node, path, navigation, options) {
  const caption = node.attrs.caption || node.attrs.title || "";
  return `<table${htmlAttrs(node)}>${caption.trim() ? `<caption>${escapeHtml(caption)}</caption>` : ""}${renderChildren(node, path, navigation, options)}</table>`;
}

function renderCallout(node, path, navigation, options) {
  const type = calloutType(node);
  const tag = type === "example" || type === "summary" ? "section" : "aside";
  const label = node.attrs.title || calloutLabel(type);
  return `<${tag}${htmlAttrsWithExtraClass(node, `nodx-callout nodx-callout--${type}`)}${calloutA11yAttr(node, label)}><p class="nodx-callout__label">${escapeHtml(label)}</p>${renderChildren(node, path, navigation, options)}</${tag}>`;
}

function calloutA11yAttr(node, label) {
  return node.attrs["aria-label"] || node.attrs["aria-labelledby"] ? "" : ` aria-label="${escapeAttr(label)}"`;
}

function calloutType(node) {
  const raw = node.type === "note" ? (node.attrs.type || "note") : node.type;
  return ["note", "info", "tip", "important", "caution", "warning", "danger", "example", "summary"].includes(raw) ? raw : "note";
}

function calloutLabel(type) {
  return {
    note: "Note",
    info: "Info",
    tip: "Tip",
    important: "Important",
    caution: "Caution",
    warning: "Warning",
    danger: "Danger",
    example: "Example",
    summary: "Summary",
  }[type] ?? "Note";
}

function isCalloutNode(node) {
  return node.type === "note" || ["info", "tip", "important", "caution", "warning", "danger", "example", "summary"].includes(node.type);
}

function wrapLayoutChildren(className, node, path, navigation, options) {
  return `<div${htmlAttrsWithExtraClass(node, className)}>${renderChildren(node, path, navigation, options)}</div>`;
}

function renderChildren(node, path, navigation, options) {
  return node.children.map((child, index) => renderNode(child, childPath(path, index), navigation, options)).join("");
}

function renderComponentTemplate(component, node, path, navigation, options) {
  const rendered = renderTemplatePart(component.template, component, node, path, navigation, options);
  const children = renderInlines(node.inlines, options) + renderChildren(node, path, navigation, options);
  return rendered.replaceAll("<p><var>vars.children</var></p>", children).replaceAll("<var>vars.children</var>", children);
}

function renderTemplatePart(part, component, node, path, navigation, options) {
  const parsed = parse(substituteTemplateVars(part, node, options.doc));
  return parsed.body.map((child, index) => renderNode(child, `${path}.template.${component.name}.${index}`, navigation, options)).join("");
}

function substituteTemplateVars(source, node, doc) {
  return source.replace(/\{\{\s*([A-Za-z0-9_.-]+)\s*\}\}/g, (_match, key) => {
    if (key === "children") return "{{children}}";
    if (key.startsWith("attrs.")) return String(node.attrs[key.slice(6)] ?? "");
    if (key.startsWith("vars.")) return String(doc?.meta?.vars?.[key.slice(5)] ?? "");
    return String(node.attrs[key] ?? "");
  });
}

function wrapChildren(tag, node, path, navigation, options) {
  return `<${tag}${htmlAttrs(node)}>${renderChildren(node, path, navigation, options)}</${tag}>`;
}

function wrapInlines(tag, node, path, navigation, options) {
  return `<${tag}${htmlAttrs(node)}>${renderInlines(node.inlines, options)}${renderChildren(node, path, navigation, options)}</${tag}>`;
}

function renderStyle(node) {
  try {
    const source = node.attrs.format === "yaml" ? yamlStyleToCss(node.text ?? "") : (node.text ?? "");
    return sanitizeStylesheet(source);
  } catch {
    return "/* NODX-E027: invalid YAML style block */";
  }
}

function renderImage(node, options) {
  const alt = node.attrs.alt ?? "";
  const raw = node.attrs.src ?? "";
  const src = options.assetResolver?.(raw, node) ?? safeImageUrl(raw, options.remoteAssets);
  if (!src) return `<span class="nodx-blocked-image">${escapeHtml(alt || "blocked image")}</span>`;
  return `<img${htmlAttrs(node)} src="${escapeAttr(src)}" alt="${escapeAttr(alt)}">`;
}

function renderToc(node, path, navigation) {
  const nav = navigation.navigations.find((candidate) => candidate.tocPath === path);
  const label = nav?.label ?? node.attrs.title ?? "Table of contents";
  const entries = nav?.entries ?? [];
  const items = entries.length
    ? entries.filter((entry) => isSafeFragmentId(entry.id)).map((entry) => `<li data-level="${escapeAttr(String(entry.level))}"><a href="#${escapeAttr(entry.id)}">${escapeHtml(entry.title)}</a></li>`).join("")
    : `<p class="nodx-toc-empty">(no entries)</p>`;
  return `<nav${htmlId(node)} aria-label="${escapeAttr(label)}"><strong>${escapeHtml(label)}</strong>${entries.length ? `<ol>${items}</ol>` : items}</nav>`;
}

function renderMediaFallback(node, path, navigation, options) {
  const raw = node.attrs.src ?? "";
  const src = safeMediaUrl(raw, options.remoteAssets);
  const fallback = mediaFallbackHtml(node, path, navigation, options, src);
  const children = renderChildrenWithoutMediaFallback(node, path, navigation, options);
  if (node.type === "media" && src) {
    return `<figure${htmlAttrs(node)}><video controls src="${escapeAttr(src)}">${fallback}</video>${children}</figure>`;
  }
  return `<figure${htmlAttrs(node)}><div class="media-fallback">${fallback}</div>${children}</figure>`;
}

function mediaFallbackHtml(node, path, navigation, options, src) {
  if (node.type === "include" && options.textAssetResolver?.(node.attrs.src ?? "")) {
    return escapeHtml(options.textAssetResolver(node.attrs.src ?? ""));
  }
  const fallbackNode = node.children.find((child) => child.type === "media-fallback");
  if (fallbackNode) {
    return renderInlines(fallbackNode.inlines ?? [], options) + renderChildren(fallbackNode, `${path}.${node.children.indexOf(fallbackNode)}`, navigation, options);
  }
  const text = node.attrs.alt ?? node.type;
  return escapeHtml(src ? text : `${text}${node.attrs.src ? " - " + node.attrs.src : ""}`);
}

function renderChildrenWithoutMediaFallback(node, path, navigation, options) {
  return (node.children ?? [])
    .map((child, i) => child.type === "media-fallback" ? "" : renderNode(child, `${path}.${i}`, navigation, options))
    .join("");
}

function renderInlines(inlines, options) {
  return inlines.map((item) => {
    if (item.type === "text") return escapeHtml(item.text);
    if (item.type === "strong" || item.type === "em" || item.type === "sub" || item.type === "sup") return `<${item.type}>${renderInlines(item.children, options)}</${item.type}>`;
    if (item.type === "mark") return `<mark${inlineAttrs(item.attrs)}>${renderInlines(item.children, options)}</mark>`;
    if (item.type === "strike") return `<s>${renderInlines(item.children, options)}</s>`;
    if (item.type === "code") return `<code>${escapeHtml(item.text)}</code>`;
    if (item.type === "math-inline") return `<code class="math-inline">${escapeHtml(item.source)}</code>`;
    if (item.type === "link") return renderLink(item, options);
    if (item.type === "span") return `<span${inlineAttrs(item.attrs)}>${renderInlines(item.children, options)}</span>`;
    if (item.type === "var") return `<var>${escapeHtml(item.namespace)}.${escapeHtml(item.name)}</var>`;
    if (item.type === "ref") return isSafeFragmentId(item.target) ? `<a href="#${escapeAttr(item.target)}">@${escapeHtml(item.target)}</a>` : `<span class="nodx-blocked-link">@${escapeHtml(item.target)}</span>`;
    if (item.type === "citation-ref" || item.type === "footnote-ref") return isSafeFragmentId(item.target) ? `<a href="#${escapeAttr(item.target)}">[${escapeHtml(item.target)}]</a>` : `<span class="nodx-blocked-link">[${escapeHtml(item.target)}]</span>`;
    if (item.type === "mention") return `<span class="mention">@${escapeHtml(item.kind)}:${escapeHtml(item.target)}</span>`;
    if (item.type === "line-break") return "<br>";
    return "";
  }).join("");
}

function renderLink(item) {
  const safe = safeLinkUrl(item.target);
  const label = renderInlines(item.label);
  if (!safe) return `<a class="nodx-blocked-link" data-blocked="${escapeAttr(item.target)}" title="Blocked unsafe URL">${label}</a>`;
  const title = item.attrs?.attrs?.title ? ` title="${escapeAttr(item.attrs.attrs.title)}"` : "";
  const rel = item.attrs?.attrs?.rel ?? "noopener noreferrer";
  return `<a href="${escapeAttr(safe)}"${title} rel="${escapeAttr(rel)}">${label}</a>`;
}

function htmlAttrs(node) {
  let out = htmlId(node);
  if (node.classes?.length) out += ` class="${escapeAttr(node.classes.join(" "))}"`;
  if (node.styles && Object.keys(node.styles).length) out += ` style="${escapeAttr(styleAttr(node.styles))}"`;
  if (node.attrs.lang) out += ` lang="${escapeAttr(node.attrs.lang)}"`;
  if (["ltr", "rtl", "auto"].includes(node.attrs.dir)) out += ` dir="${escapeAttr(node.attrs.dir)}"`;
  if (node.attrs.title) out += ` title="${escapeAttr(node.attrs.title)}"`;
  out += structuralAttrs(node);
  return out;
}

function htmlAttrsWithoutClass(node) {
  let out = htmlId(node);
  if (node.attrs.lang) out += ` lang="${escapeAttr(node.attrs.lang)}"`;
  if (["ltr", "rtl", "auto"].includes(node.attrs.dir)) out += ` dir="${escapeAttr(node.attrs.dir)}"`;
  if (node.attrs.title) out += ` title="${escapeAttr(node.attrs.title)}"`;
  out += structuralAttrs(node);
  return out;
}

function htmlAttrsWithExtraClass(node, extra) {
  let out = htmlId(node);
  out += ` class="${escapeAttr([extra, ...(node.classes ?? [])].join(" "))}"`;
  const backgroundImage = safeBackgroundImageCss(node.attrs.background);
  if ((node.styles && Object.keys(node.styles).length) || backgroundImage) {
    const style = [styleAttr(node.styles ?? {}), backgroundImage].filter(Boolean).join("; ");
    out += ` style="${escapeAttr(style)}"`;
  }
  if (node.attrs.lang) out += ` lang="${escapeAttr(node.attrs.lang)}"`;
  if (["ltr", "rtl", "auto"].includes(node.attrs.dir)) out += ` dir="${escapeAttr(node.attrs.dir)}"`;
  if (node.attrs.title) out += ` title="${escapeAttr(node.attrs.title)}"`;
  out += structuralAttrs(node);
  return out;
}

function pageStylesheet(doc) {
  const page = doc.meta?.page;
  if (!page || typeof page !== "object") return "";
  const rules = [];
  const bg = page.bg ?? page["background-color"];
  if (typeof bg === "string" && safeQuickStyleValue(bg)) rules.push(`background-color:${bg}`);
  if (typeof page.color === "string" && safeQuickStyleValue(page.color)) rules.push(`color:${page.color}`);
  const backgroundImage = safeBackgroundImageCss(page.background ?? page["background-image"]);
  if (backgroundImage) rules.push(backgroundImage);
  return rules.length ? `body{${rules.join(";")}}` : "";
}

function safeBackgroundImageCss(raw) {
  if (typeof raw !== "string" || !safeImageUrl(raw, false) || /['"()\\]/.test(raw)) return "";
  return `background-image:url('${raw}')`;
}

function safeQuickStyleValue(value) {
  return value.length <= 240 && !/[<>{};]/.test(value);
}

function structuralAttrs(node) {
  return Object.entries(node.attrs ?? {})
    .filter(([key, value]) => safeStructuralAttr(key, String(value)))
    .map(([key, value]) => ` ${key}="${escapeAttr(String(value))}"`)
    .join("");
}

function safeStructuralAttr(key, value) {
  if (key === "role") return /^[A-Za-z0-9-]+$/.test(value);
  if (key === "scope") return ["col", "row", "colgroup", "rowgroup"].includes(value);
  if (key === "align") return ["left", "center", "right", "start", "end"].includes(value);
  if (key === "valign") return ["top", "middle", "bottom", "baseline"].includes(value);
  if (key === "colspan" || key === "rowspan") return /^\d+$/.test(value) && Number(value) >= 1 && Number(value) <= 1000;
  return (key.startsWith("data-") || key.startsWith("aria-")) && value.length <= 240;
}

function inlineAttrs(attrs) {
  if (!attrs) return "";
  let out = attrs.id ? ` id="${escapeAttr(attrs.id)}"` : "";
  if (attrs.classes?.length) out += ` class="${escapeAttr(attrs.classes.join(" "))}"`;
  if (attrs.styles && Object.keys(attrs.styles).length) out += ` style="${escapeAttr(styleAttr(attrs.styles))}"`;
  if (attrs.attrs?.lang) out += ` lang="${escapeAttr(attrs.attrs.lang)}"`;
  if (["ltr", "rtl", "auto"].includes(attrs.attrs?.dir)) out += ` dir="${escapeAttr(attrs.attrs.dir)}"`;
  if (attrs.attrs?.title) out += ` title="${escapeAttr(attrs.attrs.title)}"`;
  return out;
}

function htmlId(node) {
  return node.id ? ` id="${escapeAttr(node.id)}"` : "";
}

function classAttr(node, extra) {
  return escapeAttr([...(node.classes ?? []), extra].join(" "));
}

function styleAttr(styles) {
  return Object.entries(styles).map(([key, value]) => `${key}: ${value}`).join("; ");
}

function componentDefinitions(doc) {
  return Array.isArray(doc?.meta?.components) ? doc.meta.components.filter((item) => item && typeof item.name === "string") : [];
}

function safeLinkUrl(raw) {
  return classifyUri(ReferenceKind.Link, raw).ok ? raw : null;
}

function safeImageUrl(raw, remoteAssets = false) {
  return classifyUri(ReferenceKind.Asset, raw, undefined, { remoteAssets }).ok ? raw : null;
}

function safeMediaUrl(raw, remoteAssets = false) {
  return classifyUri(ReferenceKind.MediaFallback, raw, undefined, { remoteAssets }).ok ? raw : null;
}

function documentAllowsRemoteAssets(doc) {
  const features = doc.meta?.features;
  if (features && typeof features === "object" && !Array.isArray(features) && features["remote-assets"] === true) return true;
  const profiles = doc.meta?.profiles;
  if (profiles && typeof profiles === "object" && !Array.isArray(profiles)) {
    for (const key of ["requires", "optional"]) {
      if (Array.isArray(profiles[key]) && profiles[key].includes("remote-assets")) return true;
    }
  }
  return false;
}

function childPath(prefix, index) {
  return prefix === "" ? String(index) : `${prefix}.${index}`;
}

function isSafeFragmentId(input) {
  return /^[A-Za-z_][A-Za-z0-9_.-]*$/.test(input.replace(/^#/, ""));
}

function deriveTitle(nodes) {
  for (const node of nodes) {
    if (node.type === "heading") return plainInlines(node.inlines);
    const child = deriveTitle(node.children ?? []);
    if (child) return child;
  }
  return "";
}

function plainInlines(inlines) {
  let out = "";
  for (const item of inlines) {
    switch (item.type) {
      case "text":
      case "code":
        out += item.text;
        break;
      case "math-inline":
        out += item.source;
        break;
      case "strong":
      case "em":
      case "mark":
      case "strike":
      case "sub":
      case "sup":
        out += plainInlines(item.children);
        break;
      case "link":
        out += plainInlines(item.label);
        break;
      case "span":
        out += plainInlines(item.children);
        break;
      case "var":
        out += "{{" + item.namespace + "." + item.name + "}}";
        break;
      case "ref":
      case "footnote-ref":
      case "citation-ref":
        out += item.target;
        break;
      case "mention":
        out += "@" + item.kind + ":" + item.target;
        break;
      case "line-break":
        out += " ";
        break;
    }
  }
  return out;
}

function standardTokens() {
  return "html{font-family:system-ui}html[dir=\"rtl\"]{direction:rtl}:root{--nodx-color-text:#1f2937;--nodx-color-muted:#4b5563;--nodx-color-bg:#ffffff;--nodx-color-primary:#0f766e;--nodx-color-accent:#b91c1c;--nodx-color-rule:#e5e7eb;--nodx-color-surface:transparent;--nodx-font-body:system-ui,-apple-system,BlinkMacSystemFont,\"Segoe UI\",sans-serif;--nodx-font-heading:var(--nodx-font-body);--nodx-font-mono:ui-monospace,SFMono-Regular,Menlo,monospace;--nodx-page-margin:22mm;--nodx-line-height:1.6;--nodx-block-gap:1rem}";
}

function commonStyles() {
  return [
    "h1,h2,h3,h4,h5,h6{font-family:var(--nodx-font-heading);line-height:1.25;color:var(--nodx-color-heading,#0f172a);margin-top:1.4em}",
    "p{margin:0 0 1em}pre{padding:12px;background:#f5f5f5;overflow:auto;border-radius:6px}code{font-family:var(--nodx-font-mono)}",
    "aside{border-inline-start:4px solid #b57f00;padding:8px 12px;background:#fff8e6}",
    ".nodx-callout{margin:1em 0;padding:.85em 1em;border:1px solid var(--nodx-callout-border,#d1d5db);border-inline-start-width:4px;border-radius:8px;background:var(--nodx-callout-bg,#f8fafc);color:var(--nodx-color-text)}",
    ".nodx-callout__label{margin:0 0 .35em;font-size:.78em;font-weight:750;letter-spacing:.04em;text-transform:uppercase;color:var(--nodx-callout-fg,var(--nodx-color-muted))}",
    ".nodx-callout--note{--nodx-callout-border:#94a3b8;--nodx-callout-bg:#f8fafc;--nodx-callout-fg:#475569}.nodx-callout--info{--nodx-callout-border:#38bdf8;--nodx-callout-bg:#f0f9ff;--nodx-callout-fg:#0369a1}.nodx-callout--tip{--nodx-callout-border:#2dd4bf;--nodx-callout-bg:#f0fdfa;--nodx-callout-fg:#0f766e}",
    ".nodx-callout--important{--nodx-callout-border:#a78bfa;--nodx-callout-bg:#f5f3ff;--nodx-callout-fg:#6d28d9}.nodx-callout--caution{--nodx-callout-border:#f59e0b;--nodx-callout-bg:#fffbeb;--nodx-callout-fg:#b45309}.nodx-callout--warning{--nodx-callout-border:#f97316;--nodx-callout-bg:#fff7ed;--nodx-callout-fg:#c2410c}",
    ".nodx-callout--danger{--nodx-callout-border:#ef4444;--nodx-callout-bg:#fef2f2;--nodx-callout-fg:#b91c1c}.nodx-callout--example{--nodx-callout-border:#22c55e;--nodx-callout-bg:#f0fdf4;--nodx-callout-fg:#15803d}.nodx-callout--summary{--nodx-callout-border:#64748b;--nodx-callout-bg:#f8fafc;--nodx-callout-fg:#334155}",
    "table{border-collapse:collapse;margin:0 0 1em}caption{text-align:start;font-weight:600;margin-bottom:.35em}td,th{border:1px solid #d1d5db;padding:6px 10px}thead th{background:#f3f4f6;text-align:start}",
    "figure{margin:1.5em 0}figcaption{font-size:.9em;color:var(--nodx-color-muted)}nav ol{padding-inline-start:1.5rem}nav strong{display:block;margin-bottom:.4em}",
    ".nodx-grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(12rem,1fr));gap:var(--nodx-block-gap);margin:0 0 1em}.nodx-columns{columns:2 18rem;column-gap:2rem;margin:0 0 1em}.nodx-frame{border:1px solid var(--nodx-color-rule,#e5e7eb);padding:1rem;margin:0 0 1em;border-radius:6px;background:var(--nodx-color-surface,transparent)}",
    ".nodx-blocked-link,.nodx-blocked-image{color:var(--nodx-color-accent);text-decoration:line-through}.nodx-blocked-link{cursor:not-allowed}.mention{font-variant:all-small-caps}.pagebreak{border:none;border-top:1px dashed #9ca3af;margin:2em 0}.math-inline{background:#f3f4f6;padding:1px 4px;border-radius:3px}.media-fallback{border:1px dashed #d1d5db;padding:12px;border-radius:6px;color:var(--nodx-color-muted)}",
  ].join("");
}

function docsStyles() {
  return "body.nodx-docs-layout{font:16px/1.65 var(--nodx-font-body);color:var(--nodx-color-text);margin:0;display:grid;grid-template-columns:minmax(220px,280px) minmax(0,1fr) minmax(180px,240px);gap:0;min-height:100vh}.nodx-docs-sidebar,.nodx-docs-outline{position:sticky;top:0;height:100vh;overflow:auto;padding:24px 18px;border-color:#e5e7eb}.nodx-docs-sidebar{border-inline-end:1px solid #e5e7eb;background:#f8fafc}.nodx-docs-outline{border-inline-start:1px solid #e5e7eb;background:#fff}.nodx-docs-main{min-width:0;max-width:860px;width:100%;padding:32px 32px 64px;margin:0 auto}.nodx-docs-brand{display:block;font-weight:700;color:var(--nodx-color-text);text-decoration:none;margin-bottom:18px}.nodx-docs-layout nav ol{list-style:none;padding:0;margin:0}.nodx-docs-layout nav li{margin:2px 0}.nodx-docs-layout nav li[data-level=\"2\"]{padding-inline-start:12px}.nodx-docs-layout nav li[data-level=\"3\"],.nodx-docs-layout nav li[data-level=\"4\"],.nodx-docs-layout nav li[data-level=\"5\"],.nodx-docs-layout nav li[data-level=\"6\"]{padding-inline-start:22px}.nodx-docs-layout nav a{display:block;color:#374151;text-decoration:none;border-radius:6px;padding:4px 6px}.nodx-docs-layout nav a:hover{background:#eef2ff;color:#111827}@media(max-width:920px){body.nodx-docs-layout{display:block}.nodx-docs-sidebar,.nodx-docs-outline{position:static;height:auto;border:0;border-bottom:1px solid #e5e7eb}.nodx-docs-outline{display:none}.nodx-docs-main{padding:24px 18px 48px}}";
}

function clamp(value, min, max) {
  return Math.max(min, Math.min(max, value));
}

function escapeHtml(input) {
  return String(input ?? "").replace(/[&<>]/g, (ch) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;" }[ch]));
}

function escapeAttr(input) {
  return escapeHtml(input).replace(/"/g, "&quot;").replace(/'/g, "&#x27;");
}

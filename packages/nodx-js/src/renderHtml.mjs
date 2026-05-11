import { resolveNavigation } from "./navigation.mjs";
import { classifyUri, ReferenceKind } from "./url.mjs";
import { sanitizeStylesheet, yamlStyleToCss } from "./nods.mjs";

export const THEME_NAMES = ["none", "plain", "base", "web", "print", "presentation"];

export function renderFragment(doc, options = {}) {
  const navigation = resolveNavigation(doc);
  const extraStyles = (options.stylesheets ?? []).map((css) => `<style>${sanitizeStylesheet(css)}</style>`).join("");
  return extraStyles + doc.body.map((node, index) => renderNode(node, String(index), navigation, options)).join("");
}

export function renderHtml(doc, options = {}) {
  const title = typeof doc.meta.title === "string" ? doc.meta.title : deriveTitle(doc.body);
  const lang = typeof doc.meta.language === "string" && doc.meta.language !== "und" ? ` lang="${escapeAttr(doc.meta.language)}"` : "";
  const dir = typeof doc.meta.dir === "string" && doc.meta.dir !== "auto" ? ` dir="${escapeAttr(doc.meta.dir)}"` : "";
  return `<!doctype html><html${lang}${dir}><meta charset="utf-8"><style>${themeStylesheet(doc.meta.theme)}</style>${title ? `<title>${escapeHtml(title)}</title>` : ""}<body>${renderFragment(doc, options)}</body></html>`;
}

export function themeStylesheet(theme = "base") {
  const name = THEME_NAMES.includes(theme) ? theme : "base";
  if (name === "none") return "html[dir=\"rtl\"]{direction:rtl}";
  if (name === "plain") return "html[dir=\"rtl\"]{direction:rtl}body{font:16px/1.55 system-ui,-apple-system,BlinkMacSystemFont,\"Segoe UI\",sans-serif;color:#1f2937}";
  const tokens = standardTokens();
  const common = commonStyles();
  if (name === "print") {
    return tokens + "body{font:11pt/1.55 var(--nodx-font-body);max-width:none;margin:0;color:var(--nodx-color-text)}@page{size:A4;margin:var(--nodx-page-margin)}h1,h2,h3{break-after:avoid}table,figure,aside{break-inside:avoid}.pagebreak{break-before:page;border:0;margin:0}" + common;
  }
  if (name === "presentation") {
    return tokens + "body{font:28px/1.45 var(--nodx-font-body);max-width:1100px;margin:40px auto;padding:0 28px;color:var(--nodx-color-text)}h1{font-size:2.4em}h2{font-size:1.8em}" + common;
  }
  if (name === "web") {
    return tokens + "body{font:16px/1.65 var(--nodx-font-body);max-width:960px;margin:32px auto;padding:0 18px;color:var(--nodx-color-text)}" + common;
  }
  return tokens + "body{font:16px/1.6 var(--nodx-font-body);max-width:920px;margin:32px auto;padding:0 16px;color:var(--nodx-color-text)}" + common;
}

export function renderSemanticText(doc) {
  const lines = [];
  writeSemanticNodes(doc.body, lines, []);
  return lines.join("\n").replace(/\n{3,}/g, "\n\n").trim() + "\n";
}

function writeSemanticNodes(nodes, lines, path) {
  let index = 0;
  for (const node of nodes) {
    if (["style", "pagebreak", "toc"].includes(node.type)) continue;
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
  if (node.type === "note") {
    lines.push(`Note${semanticAttrs(node)}: ${semanticNodeText(node)}`.trim());
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
    if (["style", "pagebreak", "toc"].includes(child.type)) continue;
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
  switch (node.type) {
    case "heading": {
      const level = clamp(Number(node.attrs.level ?? 1), 1, 6);
      return `<h${level}${htmlAttrs(node)}>${renderInlines(node.inlines, options)}</h${level}>`;
    }
    case "paragraph": return wrapInlines("p", node, path, navigation, options);
    case "section": return wrapChildren("section", node, path, navigation, options);
    case "note": return wrapChildren("aside", node, path, navigation, options);
    case "quote": return wrapChildren("blockquote", node, path, navigation, options);
    case "list": return wrapChildren(node.attrs.kind === "ordered" ? "ol" : "ul", node, path, navigation, options);
    case "item": return wrapInlines("li", node, path, navigation, options);
    case "code":
    case "pre": return `<pre><code${node.attrs.lang ? ` data-lang="${escapeAttr(node.attrs.lang)}"` : ""}>${escapeHtml(node.text ?? "")}</code></pre>`;
    case "math": return `<pre class="math">${escapeHtml(node.text ?? "")}</pre>`;
    case "style": return `<style>${renderStyle(node)}</style>`;
    case "table": return wrapChildren("table", node, path, navigation, options);
    case "row": return wrapChildren("tr", node, path, navigation, options);
    case "cell": return wrapInlines(node.attrs.header === "true" ? "th" : "td", node, path, navigation, options);
    case "figure": return wrapChildren("figure", node, path, navigation, options);
    case "caption": return wrapInlines("figcaption", node, path, navigation, options);
    case "image": return renderImage(node, options);
    case "form": return wrapChildren("dl", node, path, navigation, options);
    case "field": return `<div${htmlId(node)}><dt>${escapeHtml(node.attrs.label ?? node.attrs.name ?? "Field")}</dt><dd>${escapeHtml(node.attrs.value ?? "")}</dd></div>`;
    case "toc": return renderToc(node, path, navigation);
    case "pagebreak": return `<hr${htmlId(node)} class="pagebreak">`;
    case "media":
    case "embed":
    case "include": return renderMediaFallback(node, path, navigation, options);
    case "bibliography": return wrapChildren("ol", node, path, navigation, options);
    case "citation-entry": return wrapInlines("li", node, path, navigation, options);
    case "speaker-notes": return `<aside${htmlAttrsWithoutClass(node)} class="${classAttr(node, "speaker-notes")}" aria-label="Speaker notes">${renderInlines(node.inlines, options)}${renderChildren(node, path, navigation, options)}</aside>`;
    default:
      if (node.type.includes("-")) {
        return `<section${htmlAttrsWithoutClass(node)} class="${classAttr(node, "nodx-component nodx-component--fallback")}" data-component="${escapeAttr(node.type)}"><p class="nodx-component__title">${escapeHtml(node.type)} fallback</p>${renderInlines(node.inlines, options)}${renderChildren(node, path, navigation, options)}</section>`;
      }
      return wrapChildren("div", node, path, navigation, options);
  }
}

function renderChildren(node, path, navigation, options) {
  return node.children.map((child, index) => renderNode(child, childPath(path, index), navigation, options)).join("");
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
  const src = options.assetResolver?.(raw, node) ?? safeImageUrl(raw);
  if (!src) return `<span class="nodx-blocked-image">${escapeHtml(alt || "blocked image")}</span>`;
  return `<img src="${escapeAttr(src)}" alt="${escapeAttr(alt)}">`;
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
  const text = node.type === "include" && options.textAssetResolver?.(raw) ? options.textAssetResolver(raw) : `${node.attrs.alt ?? node.type}${raw ? " - " + raw : ""}`;
  return `<figure${htmlAttrs(node)}><div class="media-fallback">${escapeHtml(text)}</div>${renderChildren(node, path, navigation, options)}</figure>`;
}

function renderInlines(inlines, options) {
  return inlines.map((item) => {
    if (item.type === "text") return escapeHtml(item.text);
    if (item.type === "strong" || item.type === "em" || item.type === "mark" || item.type === "sub" || item.type === "sup") return `<${item.type}>${renderInlines(item.children, options)}</${item.type}>`;
    if (item.type === "code") return `<code>${escapeHtml(item.text)}</code>`;
    if (item.type === "math-inline") return `<code class="math-inline">${escapeHtml(item.source)}</code>`;
    if (item.type === "link") return renderLink(item, options);
    if (item.type === "span") return `<span${inlineAttrs(item.attrs)}>${renderInlines(item.children, options)}</span>`;
    if (item.type === "var") return `<var>${escapeHtml(item.namespace)}.${escapeHtml(item.name)}</var>`;
    if (item.type === "ref") return isSafeFragmentId(item.target) ? `<a href="#${escapeAttr(item.target)}">@${escapeHtml(item.target)}</a>` : `<span class="nodx-blocked-link">@${escapeHtml(item.target)}</span>`;
    if (item.type === "citation-ref" || item.type === "footnote-ref") return isSafeFragmentId(item.target) ? `<a href="#${escapeAttr(item.target)}">[${escapeHtml(item.target)}]</a>` : `<span class="nodx-blocked-link">[${escapeHtml(item.target)}]</span>`;
    if (item.type === "mention") return `<span class="mention">@${escapeHtml(item.kind)}:${escapeHtml(item.target)}</span>`;
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
  if (node.attrs.lang) out += ` lang="${escapeAttr(node.attrs.lang)}"`;
  if (["ltr", "rtl", "auto"].includes(node.attrs.dir)) out += ` dir="${escapeAttr(node.attrs.dir)}"`;
  if (node.attrs.title) out += ` title="${escapeAttr(node.attrs.title)}"`;
  return out;
}

function htmlAttrsWithoutClass(node) {
  let out = htmlId(node);
  if (node.attrs.lang) out += ` lang="${escapeAttr(node.attrs.lang)}"`;
  if (["ltr", "rtl", "auto"].includes(node.attrs.dir)) out += ` dir="${escapeAttr(node.attrs.dir)}"`;
  if (node.attrs.title) out += ` title="${escapeAttr(node.attrs.title)}"`;
  return out;
}

function inlineAttrs(attrs) {
  if (!attrs) return "";
  let out = attrs.id ? ` id="${escapeAttr(attrs.id)}"` : "";
  if (attrs.classes?.length) out += ` class="${escapeAttr(attrs.classes.join(" "))}"`;
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

function safeLinkUrl(raw) {
  return classifyUri(ReferenceKind.Link, raw).ok ? raw : null;
}

function safeImageUrl(raw) {
  return classifyUri(ReferenceKind.Asset, raw).ok ? raw : null;
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
  return inlines.map((item) => item.text ?? item.source ?? (item.children ? plainInlines(item.children) : item.label ? plainInlines(item.label) : item.target ?? "")).join("");
}

function standardTokens() {
  return "html{font-family:system-ui}html[dir=\"rtl\"]{direction:rtl}:root{--nodx-color-text:#1f2937;--nodx-color-muted:#4b5563;--nodx-color-primary:#0f766e;--nodx-color-accent:#b91c1c;--nodx-font-body:system-ui,-apple-system,BlinkMacSystemFont,\"Segoe UI\",sans-serif;--nodx-font-heading:var(--nodx-font-body);--nodx-font-mono:ui-monospace,SFMono-Regular,Menlo,monospace;--nodx-page-margin:22mm;--nodx-line-height:1.6;--nodx-block-gap:1rem}";
}

function commonStyles() {
  return "h1,h2,h3,h4,h5,h6{font-family:var(--nodx-font-heading);line-height:1.25;color:#0f172a;margin-top:1.4em}p{margin:0 0 1em}pre{padding:12px;background:#f5f5f5;overflow:auto;border-radius:6px}code{font-family:var(--nodx-font-mono)}aside{border-inline-start:4px solid #b57f00;padding:8px 12px;background:#fff8e6}table{border-collapse:collapse;margin:0 0 1em}td,th{border:1px solid #d1d5db;padding:6px 10px}thead th{background:#f3f4f6;text-align:start}figure{margin:1.5em 0}figcaption{font-size:.9em;color:var(--nodx-color-muted)}nav ol{padding-inline-start:1.5rem}nav strong{display:block;margin-bottom:.4em}.nodx-blocked-link,.nodx-blocked-image{color:var(--nodx-color-accent);text-decoration:line-through}.nodx-blocked-link{cursor:not-allowed}.mention{font-variant:all-small-caps}.pagebreak{border:none;border-top:1px dashed #9ca3af;margin:2em 0}.math-inline{background:#f3f4f6;padding:1px 4px;border-radius:3px}.media-fallback{border:1px dashed #d1d5db;padding:12px;border-radius:6px;color:var(--nodx-color-muted)}";
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

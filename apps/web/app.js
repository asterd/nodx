import {
  canonicalJson,
  isPackagedNodx,
  ncpJson,
  openStoredPackage,
  parse,
  renderFragment,
  renderSemanticText,
  themeStylesheet,
  THEME_NAMES,
  validate,
} from "../../packages/nodx-js/parser.mjs";
import { examples, findExample } from "./examples.js";

const textDecoder = new TextDecoder("utf-8", { fatal: true });
const textEncoder = new TextEncoder();

const src = document.getElementById("src");
const out = document.getElementById("render");
const outline = document.getElementById("outline");
const ast = document.getElementById("ast");
const ncp = document.getElementById("ncp");
const semantic = document.getElementById("semantic");
const diagnostics = document.getElementById("diagnostics");
const packagePane = document.getElementById("package");
const status = document.getElementById("status");
const sample = document.getElementById("sample");
const sampleKind = document.getElementById("sample-kind");
const sourceMeta = document.getElementById("source-meta");
const paged = document.getElementById("paged");
const wrap = document.getElementById("wrap");
const themeView = document.getElementById("theme-view");

let currentDoc = null;
let currentHeadings = [];
let packageState = emptyPackageState();
let themeStyleEl = null;

populateExamples();
populateThemes();

document.getElementById("load").addEventListener("click", () => loadSample(sample.value));
document.getElementById("export-html").addEventListener("click", exportHtml);
sample.addEventListener("change", () => loadSample(sample.value));
themeView.addEventListener("change", render);
paged.addEventListener("click", () => toggleButton(paged, render));
wrap.addEventListener("click", () => toggleButton(wrap, updateWrap));
src.addEventListener("input", render);

document.querySelectorAll("button[data-tab]").forEach((button) => {
  button.addEventListener("click", () => activateTab(button.dataset.tab));
});

updateWrap();
await loadSample(examples[0].id);

function populateExamples() {
  const groups = new Map();
  for (const example of examples) {
    if (!groups.has(example.group)) {
      const group = document.createElement("optgroup");
      group.label = example.group;
      groups.set(example.group, group);
      sample.append(group);
    }
    const option = document.createElement("option");
    option.value = example.id;
    option.textContent = example.label;
    groups.get(example.group).append(option);
  }
}

function populateThemes() {
  const auto = document.createElement("option");
  auto.value = "auto";
  auto.textContent = "Theme: document";
  themeView.append(auto);
  for (const name of THEME_NAMES) {
    const option = document.createElement("option");
    option.value = name;
    option.textContent = `Theme: ${name}`;
    themeView.append(option);
  }
}

async function loadSample(id) {
  const example = findExample(id);
  sample.value = example.id;
  clearPackageUrls();
  try {
    if (example.kind === "inline") {
      packageState = emptyPackageState();
      src.value = example.text;
    } else if (example.kind === "package") {
      loadPackageBytes(createStoredZip(example.files), "generated", example.label);
    } else {
      const response = await fetch(example.path);
      if (!response.ok) throw new Error(`${response.status} ${response.statusText}`);
      const bytes = new Uint8Array(await response.arrayBuffer());
      if (isPackagedNodx(bytes)) loadPackageBytes(bytes, example.path, example.label);
      else {
        packageState = emptyPackageState();
        src.value = textDecoder.decode(bytes);
      }
    }
    sampleKind.textContent = `${example.kind} example`;
    render();
  } catch (error) {
    packageState = emptyPackageState();
    src.value = `# Unable to load sample\n\n${String(error.message ?? error)}\n\nRun a static server from the repository root and open apps/web/index.html through http://localhost.`;
    sampleKind.textContent = "Load failed";
    render();
  }
}

function loadPackageBytes(bytes, origin, label) {
  const pkg = openStoredPackage(bytes);
  packageState = {
    entryPath: pkg.entryPath,
    files: pkg.files,
    manifest: pkg.manifest,
    origin,
    label,
    urls: new Map(),
    components: loadComponentRenderers(pkg.files),
    componentSources: loadComponentSources(pkg.files),
  };
  src.value = textDecoder.decode(pkg.entryBytes);
}

function render() {
  const started = performance.now();
  try {
    currentDoc = parse(src.value);
    const validation = validate(currentDoc);
    currentHeadings = collectHeadings(currentDoc.body);
    renderDocument(currentDoc);
    renderOutline();
    renderDiagnostics(validation);
    renderPackagePanel();
    ast.textContent = JSON.stringify(JSON.parse(canonicalJson(currentDoc)), null, 2);
    ncp.textContent = JSON.stringify(JSON.parse(ncpJson(currentDoc)), null, 2);
    semantic.textContent = renderSemanticText(currentDoc);
    updateMeta(currentDoc, validation, performance.now() - started);
  } catch (error) {
    currentDoc = null;
    currentHeadings = [];
    out.replaceChildren();
    outline.textContent = "No outline.";
    diagnostics.textContent = String(error.stack ?? error);
    ast.textContent = "";
    ncp.textContent = "";
    semantic.textContent = "";
    status.textContent = "Render failed";
  }
}

function renderDocument(doc) {
  applyDocumentMetadata(doc.meta, out);
  applyTheme(doc);
  const html = renderFragment(doc, {
    assetResolver: resolveAsset,
    textAssetResolver: resolveTextAsset,
    componentRenderers: packageState.components,
    stylesheets: packageStylesheets(),
  });
  out.classList.toggle("paged", isPressed(paged));
  if (!isPressed(paged)) {
    out.innerHTML = html;
    wireInternalLinks(out);
    return;
  }
  const nodes = nodesFromHtml(html);
  out.replaceChildren(...paginate(nodes));
  wireInternalLinks(out);
}

function applyTheme(doc) {
  const selected = themeView.value === "auto" ? doc.meta.theme : themeView.value;
  const theme = THEME_NAMES.includes(selected) ? selected : "base";
  if (!themeStyleEl) {
    themeStyleEl = document.createElement("style");
    themeStyleEl.id = "nodx-renderer-theme";
    document.head.append(themeStyleEl);
  }
  themeStyleEl.textContent = scopeThemeCss(themeStylesheet(theme));
  sampleKind.textContent = `theme ${theme}`;
}

function scopeThemeCss(css) {
  const rules = [];
  for (const chunk of css.split("}")) {
    const open = chunk.indexOf("{");
    if (open < 0) continue;
    const selector = chunk.slice(0, open).trim();
    const body = chunk.slice(open + 1).trim();
    if (!selector || !body) continue;
    if (selector.startsWith("@")) {
      rules.push(`${selector}{${body}}`);
      continue;
    }
    const scoped = selector.split(",").map((part) => {
      const s = part.trim();
      if (s === "body" || s === "html" || s === ":root") return ".doc";
      if (s.startsWith("body.")) return ".doc" + s.slice(4);
      if (s.startsWith("html[")) return ".doc" + s.slice(4);
      return `.doc ${s}`;
    }).join(",");
    rules.push(`${scoped}{${body}}`);
  }
  rules.push(".doc.paged{max-width:none;background:transparent;border:0;border-radius:0;padding:0;box-shadow:none}");
  return rules.join("\n");
}

function nodesFromHtml(html) {
  const template = document.createElement("template");
  template.innerHTML = html;
  return Array.from(template.content.childNodes);
}

function paginate(nodes) {
  const pages = [];
  let page = pageElement(1);
  let units = 0;
  for (const node of nodes.flatMap(splitLongNode)) {
    if (node.nodeType === Node.ELEMENT_NODE && node.classList.contains("pagebreak")) {
      pages.push(page);
      page = pageElement(pages.length + 1);
      units = 0;
      continue;
    }
    const estimate = estimateUnits(node);
    if (page.childNodes.length && units + estimate > 52) {
      pages.push(page);
      page = pageElement(pages.length + 1);
      units = 0;
    }
    page.append(node);
    units += estimate;
  }
  if (page.childNodes.length || pages.length === 0) pages.push(page);
  return pages;
}

function pageElement(number) {
  const page = document.createElement("section");
  page.className = "page";
  page.dataset.page = `Page ${number}`;
  return page;
}

function estimateUnits(node) {
  const text = node.textContent ?? "";
  if (node.nodeType === Node.ELEMENT_NODE && node.matches("h1")) return 8;
  if (node.nodeType === Node.ELEMENT_NODE && node.matches("h2,h3")) return 5;
  if (node.nodeType === Node.ELEMENT_NODE && node.matches("table,figure,pre,nav,aside")) return 12;
  return Math.max(3, Math.ceil(text.length / 180));
}

function splitLongNode(node) {
  const maxUnits = 46;
  const estimate = estimateUnits(node);
  if (estimate <= maxUnits || node.nodeType !== Node.ELEMENT_NODE || !["P", "LI"].includes(node.tagName)) {
    return [node];
  }
  const text = (node.textContent ?? "").trim();
  if (text.length < 900) return [node];
  const chunks = chunkWords(text, 950);
  return chunks.map((chunk, index) => {
    const clone = node.cloneNode(false);
    clone.textContent = chunk;
    clone.classList.add("page-fragment");
    if (index > 0) clone.removeAttribute("id");
    return clone;
  });
}

function chunkWords(text, targetChars) {
  const chunks = [];
  let current = "";
  for (const word of text.split(/\s+/)) {
    if (current && current.length + word.length + 1 > targetChars) {
      chunks.push(current);
      current = word;
    } else {
      current += current ? ` ${word}` : word;
    }
  }
  if (current) chunks.push(current);
  return chunks;
}

function loadComponentRenderers(files) {
  const renderers = {};
  for (const source of loadComponentSources(files)) {
    for (const component of source.components) {
      renderers[component.name] = ({ attrs, renderChildren, escapeHtml }) => {
        const title = component.render?.title ?? `${component.name} component`;
        const klass = component.render?.class ?? component.name;
        const safeAttrs = attrs.replace(/\sclass="[^"]*"/, "");
        return `<section${safeAttrs} class="nodx-component ${escapeHtml(klass)}" data-component="${escapeHtml(component.name)}"><p class="nodx-component__title">${escapeHtml(title)} · loaded from ${escapeHtml(source.path)}</p>${renderChildren()}</section>`;
      };
    }
  }
  return renderers;
}

function loadComponentSources(files) {
  const sources = [];
  for (const [path, bytes] of files.entries()) {
    if (!path.endsWith(".nodc")) continue;
    const text = textDecoder.decode(bytes);
    try {
      const manifest = JSON.parse(text);
      sources.push({
        path,
        text,
        components: (manifest.components ?? []).filter((component) => typeof component.name === "string"),
      });
    } catch (error) {
      sources.push({ path, text, components: [], error: String(error.message ?? error) });
    }
  }
  return sources;
}

function renderOutline() {
  if (!currentHeadings.length) {
    outline.textContent = "No headings.";
    return;
  }
  const list = document.createElement("ol");
  list.className = "outline-list";
  for (const heading of currentHeadings) {
    const item = document.createElement("li");
    item.style.marginLeft = `${Math.max(0, heading.level - 1) * 14}px`;
    const link = heading.id ? document.createElement("a") : document.createElement("span");
    if (heading.id) link.href = `#${encodeURIComponent(heading.id)}`;
    link.textContent = `H${heading.level} ${heading.text}`;
    item.append(link);
    if (heading.path) item.append(" ", badge(heading.path));
    list.append(item);
  }
  outline.replaceChildren(list);
  wireInternalLinks(outline);
}

function renderDiagnostics(items) {
  if (!items.length) {
    diagnostics.textContent = "No diagnostics.";
    return;
  }
  const list = document.createElement("ol");
  list.className = "diagnostic-list";
  for (const item of items) {
    const li = document.createElement("li");
    li.className = item.severity === "warning" ? "diagnostic-warning" : "diagnostic-error";
    li.textContent = `${item.code} ${item.severity}: ${item.message}`;
    if (item.target) li.append(" ", badge(item.target));
    list.append(li);
  }
  diagnostics.replaceChildren(list);
}

function renderPackagePanel(selected = "") {
  if (!packageState.entryPath) {
    packagePane.textContent = "No package loaded. Choose a ZIP package example to inspect manifest, entry, assets, styles and component manifests.";
    return;
  }
  const fileNames = [...packageState.files.keys()];
  const active = selected || packageState.entryPath;
  const root = document.createElement("div");
  root.append(
    line("Origin", packageState.origin),
    line("Entry", packageState.entryPath),
    line("Schema", packageState.manifest.schema ?? "unknown"),
    line("Applied package styles", [...packageState.files.keys()].filter((name) => name.endsWith(".nods")).join(", ") || "none"),
    line("Loaded component renderers", Object.keys(packageState.components).join(", ") || "none"),
  );
  if (packageState.componentSources.length) root.append(renderComponentSources());
  const grid = document.createElement("div");
  grid.className = "package-grid";
  const list = document.createElement("div");
  list.className = "package-files";
  const viewer = document.createElement("div");
  viewer.className = "package-viewer";
  for (const name of fileNames) {
    const button = document.createElement("button");
    button.type = "button";
    button.textContent = name;
    button.setAttribute("aria-pressed", String(name === active));
    button.addEventListener("click", () => renderPackagePanel(name));
    list.append(button);
  }
  renderPackageFile(viewer, active);
  grid.append(list, viewer);
  root.append(grid);
  packagePane.replaceChildren(root);
}

function renderPackageFile(viewer, name) {
  const bytes = packageState.files.get(name);
  if (!bytes) {
    viewer.textContent = "Select a package file.";
    return;
  }
  if (isImage(name)) {
    const img = document.createElement("img");
    img.src = packageUrl(name);
    img.alt = name;
    viewer.replaceChildren(img);
    return;
  }
  if (isText(name)) {
    const text = textDecoder.decode(bytes);
    if (name.endsWith(".nodc")) {
      viewer.replaceChildren(componentSourceView(name, text));
    } else {
      viewer.textContent = text;
    }
    return;
  }
  viewer.textContent = `${name}\n${bytes.byteLength} bytes\nBinary preview is not available.`;
}

function renderComponentSources() {
  const box = document.createElement("div");
  const title = document.createElement("p");
  title.innerHTML = "<strong>Component sources:</strong>";
  const list = document.createElement("ol");
  list.className = "package-list";
  for (const source of packageState.componentSources) {
    const item = document.createElement("li");
    item.textContent = `${source.path}: ${source.components.map((component) => component.name).join(", ") || "no valid components"}`;
    if (source.error) item.append(" ", badge(source.error));
    list.append(item);
  }
  box.append(title, list);
  return box;
}

function componentSourceView(name, text) {
  const root = document.createElement("div");
  const source = packageState.componentSources.find((item) => item.path === name);
  if (source) {
    const summary = document.createElement("div");
    summary.className = "component-summary";
    summary.append(line("Component manifest", name));
    summary.append(line("Registered renderers", source.components.map((component) => component.name).join(", ") || "none"));
    if (source.error) summary.append(line("Parse error", source.error));
    root.append(summary);
  }
  const pre = document.createElement("pre");
  pre.textContent = text;
  root.append(pre);
  return root;
}

function collectHeadings(nodes, path = "", outItems = []) {
  nodes.forEach((node, index) => {
    const childPath = path ? `${path}.${index}` : String(index);
    if (node.type === "heading") {
      outItems.push({
        id: node.id,
        level: Number(node.attrs.level ?? 1),
        path: childPath,
        text: plainInline(node.inlines),
      });
    }
    collectHeadings(node.children ?? [], childPath, outItems);
  });
  return outItems;
}

function plainInline(inlines) {
  return inlines.map((inline) => inline.text ?? inline.source ?? (inline.children ? plainInline(inline.children) : inline.label ? plainInline(inline.label) : inline.target ?? "")).join("");
}

function resolveAsset(raw) {
  if (packageState.files.has(raw)) return packageUrl(raw);
  if (raw.startsWith("examples/")) return `../../${raw}`;
  if (raw.startsWith("assets/")) return `../../examples/${raw}`;
  return null;
}

function resolveTextAsset(raw) {
  if (!packageState.files.has(raw)) return "";
  return isText(raw) ? textDecoder.decode(packageState.files.get(raw)).slice(0, 1200) : "";
}

function packageUrl(path) {
  if (!packageState.urls.has(path)) {
    packageState.urls.set(path, URL.createObjectURL(new Blob([packageState.files.get(path)], { type: mimeType(path) })));
  }
  return packageState.urls.get(path);
}

function mimeType(path) {
  if (path.endsWith(".svg")) return "image/svg+xml";
  if (path.endsWith(".png")) return "image/png";
  if (path.endsWith(".jpg") || path.endsWith(".jpeg")) return "image/jpeg";
  if (path.endsWith(".webp")) return "image/webp";
  if (path.endsWith(".nodx")) return "text/nodx";
  if (path.endsWith(".json") || path.endsWith(".nodc")) return "application/json";
  return "application/octet-stream";
}

function isImage(path) {
  return /\.(svg|png|jpe?g|webp)$/i.test(path);
}

function isText(path) {
  return /\.(nodx|yaml|yml|json|jsonl|txt|svg|nods|nodc|jwk|jws)$/i.test(path) || path === "mimetype";
}

function clearPackageUrls() {
  for (const url of packageState.urls.values()) URL.revokeObjectURL(url);
  packageState = emptyPackageState();
}

function emptyPackageState() {
  return { entryPath: "", files: new Map(), manifest: {}, origin: "", label: "", urls: new Map(), components: {}, componentSources: [] };
}

function wireInternalLinks(root) {
  root.querySelectorAll('a[href^="#"]').forEach((link) => {
    link.addEventListener("click", (event) => {
      const id = decodeURIComponent(link.getAttribute("href").slice(1));
      const target = id ? out.querySelector("#" + CSS.escape(id)) : null;
      if (!target) return;
      event.preventDefault();
      target.scrollIntoView({ block: "start", behavior: "smooth" });
      history.replaceState(null, "", "#" + id);
      out.focus({ preventScroll: true });
    });
  });
}

function applyDocumentMetadata(meta, container) {
  const lang = typeof meta.language === "string" && meta.language !== "und" ? meta.language : "";
  const dir = typeof meta.dir === "string" && meta.dir !== "auto" ? meta.dir : "";
  if (lang) container.setAttribute("lang", lang); else container.removeAttribute("lang");
  if (dir) container.setAttribute("dir", dir); else container.removeAttribute("dir");
}

function updateMeta(doc, validation, elapsed) {
  const bytes = textEncoder.encode(src.value).byteLength;
  const errors = validation.filter((item) => item.severity === "fatal" || item.severity === "error").length;
  const warnings = validation.filter((item) => item.severity === "warning").length;
  sourceMeta.textContent = `${src.value.split(/\r?\n/).length} lines / ${bytes} bytes`;
  status.textContent = `${doc.body.length} top-level nodes, ${currentHeadings.length} headings, ${errors} errors, ${warnings} warnings, ${elapsed.toFixed(1)} ms`;
}

function activateTab(id) {
  document.querySelectorAll("button[data-tab]").forEach((button) => {
    button.setAttribute("aria-pressed", String(button.dataset.tab === id));
  });
  for (const pane of [out, outline, diagnostics, ast, ncp, semantic, packagePane]) {
    pane.classList.toggle("hidden", pane.id !== id);
  }
}

function toggleButton(button, after) {
  button.setAttribute("aria-pressed", String(!isPressed(button)));
  after();
}

function isPressed(button) {
  return button.getAttribute("aria-pressed") === "true";
}

function updateWrap() {
  src.classList.toggle("no-wrap", !isPressed(wrap));
}

function exportHtml() {
  if (!currentDoc) return;
  const selected = themeView.value === "auto" ? currentDoc.meta.theme : themeView.value;
  const theme = THEME_NAMES.includes(selected) ? selected : "base";
  const html = `<!doctype html><meta charset="utf-8"><style>${themeStylesheet(theme)}</style>${renderFragment(currentDoc, {
    assetResolver: resolveAsset,
    textAssetResolver: resolveTextAsset,
    componentRenderers: packageState.components,
    stylesheets: packageStylesheets(),
  })}`;
  const link = document.createElement("a");
  link.href = URL.createObjectURL(new Blob([html], { type: "text/html" }));
  link.download = "nodx-render.html";
  link.click();
  setTimeout(() => URL.revokeObjectURL(link.href), 1000);
}

function line(label, value) {
  const p = document.createElement("p");
  const strong = document.createElement("strong");
  strong.textContent = `${label}:`;
  p.append(strong, " ", String(value ?? ""));
  return p;
}

function packageStylesheets() {
  const styles = [];
  for (const [name, bytes] of packageState.files.entries()) {
    if (name.endsWith(".nods")) styles.push(textDecoder.decode(bytes));
  }
  return styles;
}

function badge(text) {
  const span = document.createElement("span");
  span.className = "badge";
  span.textContent = text;
  return span;
}

function createStoredZip(files) {
  const entries = Object.entries(files).map(([name, content]) => ({
    name,
    data: typeof content === "string" ? textEncoder.encode(content) : new Uint8Array(content),
  }));
  const localParts = [];
  const centralParts = [];
  let offset = 0;
  for (const entry of entries) {
    const nameBytes = textEncoder.encode(entry.name);
    const crc = crc32(entry.data);
    const local = concat(u32(0x04034b50), u16(20), u16(0), u16(0), u16(0), u16(0), u32(crc), u32(entry.data.length), u32(entry.data.length), u16(nameBytes.length), u16(0), nameBytes, entry.data);
    localParts.push(local);
    centralParts.push(concat(u32(0x02014b50), u16(20), u16(20), u16(0), u16(0), u16(0), u16(0), u32(crc), u32(entry.data.length), u32(entry.data.length), u16(nameBytes.length), u16(0), u16(0), u16(0), u16(0), u32(0), u32(offset), nameBytes));
    offset += local.length;
  }
  const centralOffset = offset;
  const central = concat(...centralParts);
  const end = concat(u32(0x06054b50), u16(0), u16(0), u16(entries.length), u16(entries.length), u32(central.length), u32(centralOffset), u16(0));
  return concat(...localParts, central, end);
}

function crc32(bytes) {
  let crc = 0xffffffff;
  for (const byte of bytes) {
    crc ^= byte;
    for (let i = 0; i < 8; i++) crc = (crc >>> 1) ^ (0xedb88320 & -(crc & 1));
  }
  return (crc ^ 0xffffffff) >>> 0;
}

function concat(...parts) {
  const length = parts.reduce((total, part) => total + part.length, 0);
  const outBytes = new Uint8Array(length);
  let offset = 0;
  for (const part of parts) {
    outBytes.set(part, offset);
    offset += part.length;
  }
  return outBytes;
}

function u16(value) {
  return Uint8Array.of(value & 0xff, (value >>> 8) & 0xff);
}

function u32(value) {
  return Uint8Array.of(value & 0xff, (value >>> 8) & 0xff, (value >>> 16) & 0xff, (value >>> 24) & 0xff);
}

import {
  canonicalJson,
  applyPackageExtensions,
  isPackagedNodx,
  ncpJson,
  openStoredPackage,
  parse,
  renderFragment,
  renderHtml,
  renderSemanticText,
  resolveNavigation,
  themeStylesheet,
  THEME_NAMES,
  validate,
} from "../../packages/nodx-js/parser.mjs";
import { examples, findExample } from "./examples.js";

const textDecoder = new TextDecoder("utf-8", { fatal: true });
const textEncoder = new TextEncoder();

const src = document.getElementById("src");
const out = document.getElementById("render");
const variablesPane = document.getElementById("variables");
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
const uiMode = document.getElementById("ui-mode");

let currentDoc = null;
let currentPreviewDoc = null;
let currentHeadings = [];
let packageState = emptyPackageState();
let themeStyleEl = null;
let variableValues = new Map();
let remoteStylesheets = [];

populateExamples();
populateThemes();

document.getElementById("load").addEventListener("click", () => loadSample(sample.value));
document.getElementById("export-html").addEventListener("click", exportHtml);
sample.addEventListener("change", () => loadSample(sample.value));
themeView.addEventListener("change", render);
paged.addEventListener("click", () => toggleButton(paged, render));
wrap.addEventListener("click", () => toggleButton(wrap, updateWrap));
uiMode.addEventListener("click", toggleUiMode);
src.addEventListener("input", render);

document.querySelectorAll("button[data-tab]").forEach((button) => {
  button.addEventListener("click", () => activateTab(button.dataset.tab));
});

updateWrap();
applyUiMode(localStorage.getItem("nodx-ui-mode") === "dark" ? "dark" : "light");
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
    const response = await fetch(example.path);
    if (!response.ok) throw new Error(`${response.status} ${response.statusText}`);
    const bytes = new Uint8Array(await response.arrayBuffer());
    if (isPackagedNodx(bytes)) loadPackageBytes(bytes, example.path, example.label);
    else {
      packageState = emptyPackageState();
      src.value = textDecoder.decode(bytes);
    }
    remoteStylesheets = await loadRemoteStylesheets(src.value, example.path);
    sampleKind.textContent = `${example.group} / ${example.label}`;
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
    currentDoc = packageState.entryPath ? applyPackageExtensions(parse(src.value), packageState) : parse(src.value);
    currentPreviewDoc = applyVariables(currentDoc);
    const validation = validate(currentPreviewDoc);
    currentHeadings = collectHeadings(currentPreviewDoc.body);
    renderDocument(currentPreviewDoc);
    renderVariablesPanel(currentDoc);
    renderOutline();
    renderDiagnostics(validation);
    renderPackagePanel();
    ast.textContent = JSON.stringify(JSON.parse(canonicalJson(currentDoc)), null, 2);
    ncp.textContent = JSON.stringify(JSON.parse(ncpJson(currentDoc)), null, 2);
    semantic.textContent = renderSemanticText(currentPreviewDoc);
    updateMeta(currentPreviewDoc, validation, performance.now() - started);
  } catch (error) {
    currentDoc = null;
    currentPreviewDoc = null;
    currentHeadings = [];
    out.replaceChildren();
    variablesPane.textContent = "No variables.";
    outline.textContent = "No headings.";
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
  applyPageStyle(doc, out);
  const options = renderOptions();
  const html = isDocsLayout(doc) && !isPressed(paged) ? renderDocsPreview(doc, options) : renderFragment(doc, options);
  out.classList.toggle("nodx-docs-layout", isDocsLayout(doc) && !isPressed(paged));
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

function toggleUiMode() {
  const next = document.documentElement.dataset.uiMode === "dark" ? "light" : "dark";
  applyUiMode(next);
  localStorage.setItem("nodx-ui-mode", next);
}

function applyUiMode(mode) {
  document.documentElement.dataset.uiMode = mode;
  uiMode.setAttribute("aria-pressed", String(mode === "dark"));
  uiMode.textContent = mode === "dark" ? "Light" : "Dark";
}

function renderOptions() {
  return {
    assetResolver: resolveAsset,
    textAssetResolver: resolveTextAsset,
    componentRenderers: packageState.components,
    stylesheets: [...remoteStylesheets, ...packageStylesheets()],
  };
}

function renderDocsPreview(doc, options) {
  const navigation = resolveNavigation(doc);
  const entries = navigation.navigations[0]?.entries ?? currentHeadings.map((heading) => ({
    id: heading.id,
    level: heading.level,
    title: heading.text,
  }));
  return `<aside class="nodx-docs-sidebar"><a class="nodx-docs-brand" href="#">${escapeHtml(docsTitle(doc))}</a>${docsNav(entries.filter((entry) => entry.level === 1), 1)}</aside><main class="nodx-docs-main">${renderFragment(doc, options)}</main><aside class="nodx-docs-outline">${docsNav(entries.filter((entry) => entry.level > 1), 6)}</aside>`;
}

function docsNav(entries, maxLevel) {
  const items = entries
    .filter((entry) => entry.id && entry.level <= maxLevel)
    .map((entry) => `<li data-level="${escapeAttr(String(entry.level))}"><a href="#${escapeAttr(entry.id)}">${escapeHtml(entry.title)}</a></li>`)
    .join("");
  return items ? `<nav><ol>${items}</ol></nav>` : "";
}

function docsTitle(doc) {
  return typeof doc.meta.title === "string" ? doc.meta.title : currentHeadings[0]?.text ?? "Documentation";
}

function isDocsLayout(doc) {
  return doc.meta.layout === "docs" || doc.meta.theme === "docs";
}

function renderVariablesPanel(doc) {
  const refs = collectVariableRefs(doc.body);
  const keys = [...new Set([
    ...Object.keys(doc.meta.vars && typeof doc.meta.vars === "object" && !Array.isArray(doc.meta.vars) ? doc.meta.vars : {}).map((name) => `vars.${name}`),
    ...refs.map((item) => variableKey(item)),
  ])].sort();
  if (!keys.length) {
    variablesPane.textContent = "No variables in this document.";
    return;
  }
  const root = document.createElement("div");
  root.className = "variables-panel";
  const table = document.createElement("table");
  table.className = "variables-table";
  const thead = document.createElement("thead");
  const headRow = document.createElement("tr");
  for (const label of ["Variable", "Value", "Source"]) {
    const th = document.createElement("th");
    th.textContent = label;
    headRow.append(th);
  }
  thead.append(headRow);
  table.append(thead);
  const tbody = document.createElement("tbody");
  for (const key of keys) {
    const [namespace, name] = splitVariableKey(key);
    const row = document.createElement("tr");
    const nameCell = document.createElement("td");
    nameCell.textContent = key;
    const valueCell = document.createElement("td");
    const input = document.createElement("input");
    input.type = "text";
    input.value = variableValues.has(key) ? variableValues.get(key) : String(defaultVariableValue(doc.meta, namespace, name) ?? "");
    input.dataset.variable = key;
    input.addEventListener("input", () => {
      variableValues.set(key, input.value);
      const applied = applyVariables(currentDoc);
      currentPreviewDoc = applied;
      currentHeadings = collectHeadings(applied.body);
      renderDocument(applied);
      renderOutline();
      semantic.textContent = renderSemanticText(applied);
      const validation = validate(applied);
      renderDiagnostics(validation);
      updateMeta(applied, validation, 0);
    });
    valueCell.append(input);
    const sourceCell = document.createElement("td");
    sourceCell.textContent = variableValues.has(key) ? "playground" : defaultVariableValue(doc.meta, namespace, name) === undefined ? "empty" : "front matter";
    row.append(nameCell, valueCell, sourceCell);
    tbody.append(row);
  }
  table.append(tbody);
  const actions = document.createElement("div");
  actions.className = "variables-actions";
  const clear = document.createElement("button");
  clear.type = "button";
  clear.textContent = "Reset overrides";
  clear.addEventListener("click", () => {
    for (const key of keys) variableValues.delete(key);
    render();
  });
  actions.append(clear);
  root.append(table, actions);
  variablesPane.replaceChildren(root);
}

function applyVariables(doc) {
  const clone = structuredClone(doc);
  const metaVars = clone.meta.vars && typeof clone.meta.vars === "object" && !Array.isArray(clone.meta.vars) ? clone.meta.vars : {};
  clone.meta.vars = { ...metaVars };
  for (const [key, value] of variableValues.entries()) {
    const [namespace, name] = splitVariableKey(key);
    if (namespace === "vars") clone.meta.vars[name] = value;
  }
  clone.body = resolveVariableNodes(clone.body, clone.meta);
  return clone;
}

function resolveVariableNodes(nodes, meta) {
  return nodes.map((node) => ({
    ...node,
    children: resolveVariableNodes(node.children ?? [], meta),
    inlines: resolveVariableInlines(node.inlines ?? [], meta),
  }));
}

function resolveVariableInlines(inlines, meta) {
  return inlines.map((inline) => {
    if (inline.type === "var") {
      const value = resolveVariableValue(meta, inline.namespace, inline.name);
      return value === undefined ? inline : { text: String(value), type: "text" };
    }
    if (inline.children) return { ...inline, children: resolveVariableInlines(inline.children, meta) };
    if (inline.label) return { ...inline, label: resolveVariableInlines(inline.label, meta) };
    return inline;
  });
}

function resolveVariableValue(meta, namespace, name) {
  const key = `${namespace}.${name}`;
  if (variableValues.has(key)) return variableValues.get(key);
  return defaultVariableValue(meta, namespace, name);
}

function defaultVariableValue(meta, namespace, name) {
  if (namespace === "vars" && meta.vars && typeof meta.vars === "object" && !Array.isArray(meta.vars)) return meta.vars[name];
  if (namespace === "meta") return meta[name];
  return undefined;
}

function collectVariableRefs(nodes, outItems = []) {
  for (const node of nodes) {
    collectVariableInlineRefs(node.inlines ?? [], outItems);
    collectVariableRefs(node.children ?? [], outItems);
  }
  return outItems;
}

function collectVariableInlineRefs(inlines, outItems) {
  for (const inline of inlines) {
    if (inline.type === "var") outItems.push(inline);
    if (inline.children) collectVariableInlineRefs(inline.children, outItems);
    if (inline.label) collectVariableInlineRefs(inline.label, outItems);
  }
}

function variableKey(item) {
  return `${item.namespace}.${item.name}`;
}

function splitVariableKey(key) {
  const dot = key.indexOf(".");
  return dot < 0 ? ["vars", key] : [key.slice(0, dot), key.slice(dot + 1)];
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
}

function applyPageStyle(doc, element) {
  const page = doc.meta?.page;
  const bg = page && typeof page === "object" ? page.bg ?? page["background-color"] : undefined;
  const color = page && typeof page === "object" ? page.color : undefined;
  const hasBg = typeof bg === "string" && safeQuickStyleValue(bg);
  const hasColor = typeof color === "string" && safeQuickStyleValue(color);
  element.classList.toggle("page-bg-styled", hasBg);
  element.classList.toggle("page-bg-adaptive", !hasBg);
  element.classList.toggle("page-color-styled", hasColor);
  element.classList.toggle("page-color-adaptive", !hasColor);
  element.style.backgroundColor = hasBg ? bg : "transparent";
  element.style.color = hasColor ? color : "var(--ink)";
  element.style.setProperty("--nodx-playground-page-color", hasColor ? color : "inherit");
}

function safeQuickStyleValue(value) {
  return value.length <= 240 && !/[<>{};]/.test(value) && !/expression\(|javascript:|vbscript:|@import|url\(/i.test(value);
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
  rules.push(".doc.page-bg-adaptive{background:transparent}");
  rules.push(".doc.page-color-adaptive{color:var(--ink);--nodx-color-text:var(--ink);--nodx-color-muted:var(--muted);--nodx-color-rule:var(--line);--nodx-color-bg:transparent;--nodx-color-surface:color-mix(in srgb,var(--panel) 72%,transparent)}");
  rules.push(".doc.page-color-adaptive h1,.doc.page-color-adaptive h2,.doc.page-color-adaptive h3,.doc.page-color-adaptive h4,.doc.page-color-adaptive h5,.doc.page-color-adaptive h6{color:var(--ink)}");
  rules.push(".doc.page-bg-adaptive figure,.doc.page-bg-adaptive nav,.doc.page-bg-adaptive .nodx-frame{background:color-mix(in srgb,var(--panel) 78%,transparent);border-color:var(--line)}");
  rules.push(".doc.page-bg-adaptive aside{background:color-mix(in srgb,var(--warn) 12%,transparent)}");
  rules.push(".doc.page-color-styled h1,.doc.page-color-styled h2,.doc.page-color-styled h3,.doc.page-color-styled h4,.doc.page-color-styled h5,.doc.page-color-styled h6{color:var(--nodx-playground-page-color)}");
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

async function loadRemoteStylesheets(source, examplePath) {
  let doc;
  try {
    doc = parse(source);
  } catch {
    return [];
  }
  const entries = Array.isArray(doc.meta.remoteStylesheets) ? doc.meta.remoteStylesheets : [];
  const stylesheets = [];
  for (const entry of entries) {
    const loaded = await fetchRemoteStylesheet(entry, examplePath);
    if (loaded) stylesheets.push(loaded);
  }
  return stylesheets;
}

async function fetchRemoteStylesheet(entry, examplePath) {
  const href = typeof entry === "string" ? entry : entry?.href ?? entry?.url;
  const fallback = typeof entry === "object" ? entry.fallback : "";
  for (const candidate of [href, fallbackUrl(examplePath, fallback)].filter(Boolean)) {
    try {
      const response = await fetch(candidate);
      if (response.ok) return await response.text();
    } catch {
      // Try the next declared source.
    }
  }
  return "";
}

function fallbackUrl(examplePath, fallback) {
  if (!fallback) return "";
  if (/^https?:\/\//i.test(fallback)) return fallback;
  return new URL(fallback, new URL(examplePath, window.location.href)).href;
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
  remoteStylesheets = [];
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
  for (const pane of [out, variablesPane, outline, diagnostics, ast, ncp, semantic, packagePane]) {
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
  const doc = currentPreviewDoc ?? currentDoc;
  if (!doc) return;
  const selected = themeView.value === "auto" ? doc.meta.theme : themeView.value;
  const theme = THEME_NAMES.includes(selected) ? selected : "base";
  const html = renderHtml({ ...doc, meta: { ...doc.meta, theme } }, renderOptions());
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
  const themePaths = new Set((packageState.manifest.themes ?? []).map((item) => typeof item === "string" ? item : item.path).filter(Boolean));
  for (const [name, bytes] of packageState.files.entries()) {
    if (name.endsWith(".nods") && !themePaths.has(name)) styles.push(textDecoder.decode(bytes));
  }
  return styles;
}

function badge(text) {
  const span = document.createElement("span");
  span.className = "badge";
  span.textContent = text;
  return span;
}

function escapeHtml(input) {
  return String(input ?? "").replace(/[&<>]/g, (ch) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;" }[ch]));
}

function escapeAttr(input) {
  return escapeHtml(input).replace(/"/g, "&quot;").replace(/'/g, "&#x27;");
}

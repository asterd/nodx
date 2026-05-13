const vscode = require("vscode");
const path = require("path");
const crypto = require("crypto");

const textDecoder = new TextDecoder("utf-8", { fatal: false });
const textEncoder = new TextEncoder();
const MIMETYPE = "application/nodx+zip";
const IMAGE_MIME_BY_EXT = new Map([
  [".png", "image/png"],
  [".jpg", "image/jpeg"],
  [".jpeg", "image/jpeg"],
  [".webp", "image/webp"],
  [".gif", "image/gif"],
  [".svg", "image/svg+xml"],
]);
const MEDIA_MIME_BY_EXT = new Map([
  [".mp4", "video/mp4"],
  [".webm", "video/webm"],
  [".mp3", "audio/mpeg"],
  [".wav", "audio/wav"],
  [".ogg", "audio/ogg"],
  ...IMAGE_MIME_BY_EXT,
]);

let nodxRuntimePromise;
const diagnosticCollection = vscode.languages.createDiagnosticCollection("nodx");

function activate(context) {
  const provider = new NodxEditorProvider(context);
  context.subscriptions.push(
    vscode.window.registerCustomEditorProvider(NodxEditorProvider.viewType, provider, {
      webviewOptions: { retainContextWhenHidden: true },
      supportsMultipleEditorsPerDocument: false,
    }),
    diagnosticCollection,
    vscode.languages.registerCompletionItemProvider("nodx", completionProvider(), ":", "{", "#", "@"),
    vscode.commands.registerCommand("nodx.openEditor", openEditor),
    vscode.commands.registerCommand("nodx.openPreviewToSide", openPreviewToSide),
    vscode.commands.registerCommand("nodx.openSemanticPreview", openSemanticPreview),
    vscode.commands.registerCommand("nodx.showInfo", showInfoForActiveDocument),
    vscode.commands.registerCommand("nodx.enableRemoteAssets", enableRemoteAssetsForActiveDocument),
    vscode.commands.registerCommand("nodx.createPlain", createPlainDocument),
    vscode.commands.registerCommand("nodx.createPackage", createPackageDocument),
  );
}

function deactivate() {}

async function runtime() {
  if (!nodxRuntimePromise) {
    const runtimePath = vscode.Uri.file(path.join(__dirname, "vendor", "nodx-js", "src", "index.mjs")).toString();
    nodxRuntimePromise = import(runtimePath);
  }
  return nodxRuntimePromise;
}

class NodxEditorProvider {
  static viewType = "nodx.editor";

  constructor(context) {
    this.context = context;
    this._onDidChangeCustomDocument = new vscode.EventEmitter();
    this.onDidChangeCustomDocument = this._onDidChangeCustomDocument.event;
  }

  async openCustomDocument(uri) {
    return NodxDocument.create(uri);
  }

  async resolveCustomEditor(document, webviewPanel) {
    webviewPanel.webview.options = { enableScripts: true };
    webviewPanel.webview.html = webviewHtml(webviewPanel.webview);
    const refresh = async () => {
      webviewPanel.webview.postMessage({ type: "state", state: await document.viewState() });
      updateDiagnostics(document.uri, document.diagnostics);
    };
    const disposable = document.onDidChange((event) => this._onDidChangeCustomDocument.fire(event));
    webviewPanel.onDidDispose(() => disposable.dispose());
    webviewPanel.webview.onDidReceiveMessage(async (message) => {
      try {
        if (message.type === "ready") await refresh();
        if (message.type === "edit") {
          await document.updateFile(message.path, message.text);
          updateDiagnostics(document.uri, document.diagnostics);
        }
        if (message.type === "mode") {
          document.mode = message.mode;
          await refresh();
        }
        if (message.type === "info") {
          webviewPanel.webview.postMessage({ type: "info", info: await document.info() });
        }
        if (message.type === "addAssetFile") {
          const result = await document.addAssetFromFile();
          if (result) document.mode = "source";
          await refresh();
          if (result) webviewPanel.webview.postMessage({ type: "assetAdded", asset: result });
        }
        if (message.type === "addAssetUrl") {
          const result = await document.addAssetFromUrl();
          if (result) document.mode = "source";
          await refresh();
          if (result) webviewPanel.webview.postMessage({ type: "assetAdded", asset: result });
        }
        if (message.type === "enableRemoteAssets") {
          await document.enableRemoteAssets();
          document.mode = "source";
          await refresh();
        }
        if (message.type === "save") await vscode.commands.executeCommand("workbench.action.files.save");
      } catch (error) {
        vscode.window.showErrorMessage(errorMessage(error));
      }
    });
    await refresh();
  }

  saveCustomDocument(document) {
    return document.save();
  }

  saveCustomDocumentAs(document, destination) {
    return document.saveAs(destination);
  }

  revertCustomDocument(document) {
    return document.revert();
  }

  backupCustomDocument(document, context) {
    return document.backup(context.destination);
  }
}

class NodxDocument {
  constructor(uri, bytes, parsed) {
    this.uri = uri;
    this.originalBytes = bytes;
    this.isPackage = parsed.isPackage;
    this.files = parsed.files;
    this.hiddenFiles = parsed.hiddenFiles;
    this.manifest = parsed.manifest;
    this.entryPath = parsed.entryPath;
    this.mode = "preview";
    this.contentChanged = false;
    this.diagnostics = [];
    this.disposables = [];
    this._onDidChange = new vscode.EventEmitter();
    this.onDidChange = this._onDidChange.event;
  }

  static async create(uri) {
    const bytes = await vscode.workspace.fs.readFile(uri);
    return new NodxDocument(uri, bytes, await parseBytes(bytes));
  }

  async updateFile(filePath, text) {
    const file = this.files.find((item) => item.path === filePath);
    if (!file) throw new Error("Unknown editable package entry: " + filePath);
    file.text = text;
    this.contentChanged = true;
    await this.recompute();
    this._onDidChange.fire({
      document: this,
      undo: async () => {},
      redo: async () => {},
    });
  }

  async recompute() {
    const rt = await runtime();
    try {
      const text = this.entryText();
      const doc = this.isPackage
        ? rt.applyPackageExtensions(rt.parse(text), this.packageLike())
        : rt.parse(text);
      this.renderedHtml = rt.renderHtml(doc, { assetResolver: (raw) => this.resolvePreviewAsset(raw) });
      this.semanticText = rt.renderSemanticText(doc);
      this.diagnostics = rt.validate(doc);
    } catch (error) {
      this.renderedHtml = errorHtml(error);
      this.semanticText = errorMessage(error);
      this.diagnostics = [diagnosticFromError(error)];
    }
  }

  async viewState() {
    await this.recompute();
    return {
      mode: this.mode,
      isPackage: this.isPackage,
      entryPath: this.entryPath,
      files: this.files.map((file) => ({ path: file.path, text: file.text, kind: file.kind })),
      assets: this.assetList(),
      html: this.renderedHtml,
      semantic: this.semanticText,
      diagnostics: this.diagnostics,
    };
  }

  async info() {
    const base = {
      fileName: path.basename(this.uri.fsPath),
      representation: this.isPackage ? "nodx zip package" : "plain nodx",
      entryPath: this.entryPath,
      headers: frontMatterHeaders(this.entryText()),
      package: null,
      signature: { present: false, verified: false, reason: "No signature declared." },
    };
    if (!this.isPackage) return base;
    const rt = await runtime();
    try {
      const pkg = rt.openStoredPackage(await this.toBytes());
      base.package = {
        integrity: "verified",
        entries: pkg.manifest.entries.map((entry) => ({
          path: entry.path,
          size: entry.size,
          sha256: entry.sha256,
        })),
      };
      if (pkg.manifest.signature) {
        base.signature = {
          present: true,
          path: pkg.manifest.signature,
          verified: false,
          reason: "Package digests are verified, but no trusted ES256 key policy is configured in this extension.",
        };
      }
    } catch (error) {
      base.package = { integrity: "invalid", error: errorMessage(error), entries: [] };
    }
    return base;
  }

  entryText() {
    return this.files.find((file) => file.path === this.entryPath)?.text ?? this.files[0]?.text ?? "";
  }

  packageLike() {
    const files = new Map();
    for (const file of this.files) files.set(file.path, textEncoder.encode(file.text));
    for (const [filePath, data] of this.hiddenFiles ?? []) files.set(filePath, data);
    return { entryPath: this.entryPath, entryBytes: files.get(this.entryPath), files, manifest: this.manifest ?? { components: [], themes: [] } };
  }

  assetList() {
    if (!this.isPackage) return [];
    return [...(this.hiddenFiles ?? []).keys()]
      .filter((filePath) => filePath.startsWith("assets/") || filePath.startsWith("media/"))
      .sort()
      .map((filePath) => ({ path: filePath, mime: mimeForPath(filePath) }));
  }

  resolvePreviewAsset(raw) {
    if (!this.isPackage || typeof raw !== "string") return null;
    const normalized = normalizeAssetPath(raw);
    if (!normalized) return null;
    const bytes = this.hiddenFiles?.get(normalized) ?? this.packageLike().files.get(normalized);
    if (!bytes) return null;
    const mime = IMAGE_MIME_BY_EXT.get(path.extname(normalized).toLowerCase());
    if (!mime) return null;
    return `data:${mime};base64,${Buffer.from(bytes).toString("base64")}`;
  }

  async addAssetFromFile() {
    this.ensurePackageForAssets();
    const picked = await vscode.window.showOpenDialog({
      canSelectFiles: true,
      canSelectFolders: false,
      canSelectMany: false,
      openLabel: "Add asset to NODX package",
    });
    if (!picked?.length) return null;
    const source = picked[0];
    const bytes = await vscode.workspace.fs.readFile(source);
    const assetPath = this.addHiddenAsset(path.basename(source.fsPath), bytes);
    return this.assetInsertResult(assetPath);
  }

  async addAssetFromUrl() {
    this.ensurePackageForAssets();
    const raw = await vscode.window.showInputBox({
      title: "Add remote asset to NODX package",
      prompt: "Remote URLs are downloaded into assets/ and referenced by package-relative path.",
      placeHolder: "https://example.com/image.png",
      validateInput: (value) => /^https?:\/\//i.test(value.trim()) ? null : "Use an absolute http or https URL.",
    });
    if (!raw) return null;
    const url = new URL(raw.trim());
    const response = await fetch(url);
    if (!response.ok) throw new Error(`Asset download failed: HTTP ${response.status}`);
    const arrayBuffer = await response.arrayBuffer();
    const bytes = new Uint8Array(arrayBuffer);
    const name = basenameFromUrl(url, response.headers.get("content-type"));
    const assetPath = this.addHiddenAsset(name, bytes);
    return this.assetInsertResult(assetPath);
  }

  async enableRemoteAssets() {
    const next = ensureRemoteAssetsFeature(this.entryText());
    if (next === this.entryText()) return;
    await this.updateFile(this.entryPath, next);
  }

  addHiddenAsset(name, bytes) {
    const assetPath = uniqueAssetPath(this.hiddenFiles ?? new Map(), this.files, name);
    if (!this.hiddenFiles) this.hiddenFiles = new Map();
    this.hiddenFiles.set(assetPath, bytes);
    this.contentChanged = true;
    this._onDidChange.fire({ document: this, undo: async () => {}, redo: async () => {} });
    return assetPath;
  }

  ensurePackageForAssets() {
    if (this.isPackage) return;
    const text = this.entryText();
    this.isPackage = true;
    this.entryPath = "content/document.nodx";
    this.files = [{ path: this.entryPath, text, kind: "content" }];
    this.hiddenFiles = new Map();
    this.manifest = { schema: "nodx-package/1.0", entry: this.entryPath, entries: [], components: [], themes: [] };
    this.contentChanged = true;
  }

  async assetInsertResult(assetPath) {
    const picked = await vscode.window.showQuickPick([
      { label: "Image block", value: imageSnippet(assetPath) },
      { label: "Media block", value: mediaSnippet(assetPath) },
      { label: "Embed block", value: embedSnippet(assetPath) },
      { label: "Path only", value: assetPath },
    ], { title: "Insert asset reference" });
    return { path: assetPath, snippet: picked?.value ?? assetPath, mime: mimeForPath(assetPath) };
  }

  async toBytes() {
    if (!this.isPackage) return textEncoder.encode(this.entryText());
    return buildPackageBytes(this.files, this.manifest, this.entryPath, this.hiddenFiles, { dropSignature: this.contentChanged });
  }

  async save() {
    await vscode.workspace.fs.writeFile(this.uri, await this.toBytes());
  }

  async saveAs(destination) {
    await vscode.workspace.fs.writeFile(destination, await this.toBytes());
  }

  async revert() {
    const fresh = await parseBytes(await vscode.workspace.fs.readFile(this.uri));
    this.isPackage = fresh.isPackage;
    this.files = fresh.files;
    this.hiddenFiles = fresh.hiddenFiles;
    this.manifest = fresh.manifest;
    this.entryPath = fresh.entryPath;
    this.contentChanged = false;
    this._onDidChange.fire({ document: this, undo: async () => {}, redo: async () => {} });
  }

  async backup(destination) {
    await vscode.workspace.fs.writeFile(destination, await this.toBytes());
    return { id: destination.toString(), delete: async () => vscode.workspace.fs.delete(destination).then(undefined, () => undefined) };
  }

  dispose() {
    this.disposables.forEach((item) => item.dispose());
  }
}

async function parseBytes(bytes) {
  const rt = await runtime();
  if (rt.isPackagedNodx(bytes)) {
    const pkg = rt.openStoredPackage(bytes);
    const editable = [];
    for (const [filePath, data] of pkg.files.entries()) {
      if (isEditablePackagePath(filePath)) editable.push({ path: filePath, text: textDecoder.decode(data), kind: packageKind(filePath) });
    }
    if (!editable.some((file) => file.path === pkg.entryPath)) {
      editable.unshift({ path: pkg.entryPath, text: textDecoder.decode(pkg.entryBytes), kind: "content" });
    }
    return {
      isPackage: true,
      entryPath: pkg.entryPath,
      manifest: pkg.manifest,
      hiddenFiles: new Map([...pkg.files.entries()].filter(([filePath]) => !isEditablePackagePath(filePath) && filePath !== "mimetype" && filePath !== "manifest.yaml")),
      files: editable.sort((a, b) => sortEditable(a, b, pkg.entryPath)),
    };
  }
  return {
    isPackage: false,
    entryPath: "document.nodx",
    manifest: null,
    hiddenFiles: new Map(),
    files: [{ path: "document.nodx", text: textDecoder.decode(bytes), kind: "content" }],
  };
}

function isEditablePackagePath(filePath) {
  return filePath.endsWith(".nodx") || filePath.endsWith(".nods") || filePath.endsWith(".nodc");
}

function packageKind(filePath) {
  if (filePath.endsWith(".nods")) return "style";
  if (filePath.endsWith(".nodc") || filePath.startsWith("components/")) return "component";
  return "content";
}

function sortEditable(a, b, entryPath) {
  if (a.path === entryPath) return -1;
  if (b.path === entryPath) return 1;
  return a.path.localeCompare(b.path);
}

function buildPackageBytes(editableFiles, manifest, entryPath, hiddenFiles = new Map(), options = {}) {
  const entryMap = new Map(editableFiles.map((file) => [file.path, textEncoder.encode(file.text)]));
  for (const [filePath, data] of hiddenFiles) entryMap.set(filePath, data);
  const components = componentPaths(editableFiles, manifest);
  const themes = themePaths(editableFiles, manifest);
  const allEntries = Array.from(entryMap.entries()).sort(([a], [b]) => sortPackagePath(a, b, entryPath));
  const manifestText = serializeManifest(entryPath, manifest, allEntries, components, themes, options);
  return writeStoredZip([
    ["mimetype", textEncoder.encode(MIMETYPE)],
    ["manifest.yaml", textEncoder.encode(manifestText)],
    ...allEntries,
  ]);
}

function componentPaths(files, manifest) {
  const declared = manifestPaths(manifest?.components);
  const found = files.filter((file) => file.kind === "component" && file.path.endsWith(".nodx")).map((file) => file.path);
  return unique([...declared, ...found]).filter((filePath) => files.some((file) => file.path === filePath));
}

function themePaths(files, manifest) {
  const declared = manifestPaths(manifest?.themes);
  const found = files.filter((file) => file.kind === "style" && (file.path.startsWith("themes/") || file.path.startsWith("styles/"))).map((file) => file.path);
  return unique([...declared, ...found]).filter((filePath) => files.some((file) => file.path === filePath));
}

function serializeManifest(entryPath, original, entries, components, themes, options = {}) {
  const lines = ["schema: nodx-package/1.0", `entry: ${entryPath}`];
  if (original?.signature && !options.dropSignature) lines.push(`signature: ${original.signature}`);
  if (components.length) lines.push("components:", ...components.map((item) => `  - path: ${item}`));
  if (themes.length) lines.push("themes:", ...themes.map((item) => `  - path: ${item}`));
  lines.push("entries:");
  for (const [filePath, data] of entries) {
    lines.push(`  - path: ${filePath}`, `    size: ${data.length}`, `    sha256: ${sha256(data)}`);
  }
  return lines.join("\n") + "\n";
}

function writeStoredZip(entries) {
  let offset = 0;
  const locals = [];
  const centrals = [];
  for (const [name, data] of entries) {
    const nameBytes = textEncoder.encode(name);
    const crc = crc32(data);
    const local = concat([
      u32(0x04034b50), u16(20), u16(0), u16(0), u16(0), u16(0),
      u32(crc), u32(data.length), u32(data.length), u16(nameBytes.length), u16(0),
      nameBytes, data,
    ]);
    locals.push(local);
    centrals.push(concat([
      u32(0x02014b50), u16(20), u16(20), u16(0), u16(0), u16(0), u16(0),
      u32(crc), u32(data.length), u32(data.length), u16(nameBytes.length), u16(0), u16(0),
      u16(0), u16(0), u32(0x81a40000), u32(offset), nameBytes,
    ]));
    offset += local.length;
  }
  const centralOffset = offset;
  const central = concat(centrals);
  const eocd = concat([
    u32(0x06054b50), u16(0), u16(0), u16(entries.length), u16(entries.length),
    u32(central.length), u32(centralOffset), u16(0),
  ]);
  return concat([...locals, central, eocd]);
}

function concat(parts) {
  const total = parts.reduce((sum, part) => sum + part.length, 0);
  const out = new Uint8Array(total);
  let offset = 0;
  for (const part of parts) {
    out.set(part, offset);
    offset += part.length;
  }
  return out;
}

function u16(value) {
  const out = new Uint8Array(2);
  new DataView(out.buffer).setUint16(0, value, true);
  return out;
}

function u32(value) {
  const out = new Uint8Array(4);
  new DataView(out.buffer).setUint32(0, value >>> 0, true);
  return out;
}

function crc32(data) {
  let crc = 0xffffffff;
  for (const byte of data) {
    crc = (crc >>> 8) ^ CRC_TABLE[(crc ^ byte) & 0xff];
  }
  return (crc ^ 0xffffffff) >>> 0;
}

const CRC_TABLE = (() => {
  const table = [];
  for (let i = 0; i < 256; i += 1) {
    let c = i;
    for (let k = 0; k < 8; k += 1) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    table[i] = c >>> 0;
  }
  return table;
})();

function sha256(data) {
  return "sha256-" + crypto.createHash("sha256").update(data).digest("base64url");
}

function sortPackagePath(a, b, entryPath) {
  if (a === entryPath) return -1;
  if (b === entryPath) return 1;
  return a.localeCompare(b);
}

function manifestPaths(items) {
  return Array.isArray(items) ? items.map((item) => typeof item === "string" ? item : item?.path).filter(Boolean) : [];
}

function unique(items) {
  return [...new Set(items)];
}

function normalizeAssetPath(raw) {
  if (!raw || raw.startsWith("/") || raw.includes("\\") || raw.includes("..")) return null;
  if (/^[a-z][a-z0-9+.-]*:/i.test(raw)) return null;
  return raw.split("/").filter(Boolean).join("/");
}

function uniqueAssetPath(hiddenFiles, files, name) {
  const clean = safeAssetName(name);
  const existing = new Set([...hiddenFiles.keys(), ...files.map((file) => file.path)]);
  const ext = path.extname(clean);
  const stem = path.basename(clean, ext);
  let candidate = `assets/${clean}`;
  let index = 2;
  while (existing.has(candidate)) {
    candidate = `assets/${stem}-${index}${ext}`;
    index += 1;
  }
  return candidate;
}

function safeAssetName(name) {
  const fallback = "asset.bin";
  const base = path.basename(String(name || fallback)).replace(/[^A-Za-z0-9._-]+/g, "-").replace(/^-+|-+$/g, "");
  return base || fallback;
}

function basenameFromUrl(url, contentType) {
  const pathname = decodeURIComponent(url.pathname || "");
  const base = safeAssetName(path.basename(pathname));
  if (base && base !== "asset.bin" && path.extname(base)) return base;
  const ext = extensionForContentType(contentType) ?? ".bin";
  return `asset${ext}`;
}

function extensionForContentType(contentType) {
  const mime = String(contentType ?? "").split(";", 1)[0].trim().toLowerCase();
  for (const [ext, candidate] of MEDIA_MIME_BY_EXT.entries()) {
    if (candidate === mime) return ext;
  }
  if (mime === "image/svg+xml") return ".svg";
  if (mime === "application/pdf") return ".pdf";
  return null;
}

function mimeForPath(filePath) {
  return MEDIA_MIME_BY_EXT.get(path.extname(filePath).toLowerCase()) ?? "application/octet-stream";
}

function imageSnippet(assetPath) {
  return `:::image {src="${assetPath}" alt="Describe this image"}\n:::\n`;
}

function mediaSnippet(assetPath) {
  return `:::media {src="${assetPath}" alt="Describe this media"}\n:::media-fallback\nDescribe this media for renderers that do not play media.\n:::\n:::\n`;
}

function embedSnippet(assetPath) {
  return `:::embed {src="${assetPath}" alt="Describe this embedded asset"}\n:::media-fallback\nDescribe this embedded asset for renderers that cannot display it.\n:::\n:::\n`;
}

async function openEditor(uri) {
  const target = uri ?? vscode.window.activeTextEditor?.document.uri;
  if (!target) return;
  await vscode.commands.executeCommand("vscode.openWith", target, NodxEditorProvider.viewType);
}

async function openPreviewToSide(uri) {
  const target = uri ?? vscode.window.activeTextEditor?.document.uri;
  if (!target) return;
  const bytes = await vscode.workspace.fs.readFile(target);
  const parsed = await parseBytes(bytes);
  const doc = new NodxDocument(target, bytes, parsed);
  const state = await doc.viewState();
  const panel = vscode.window.createWebviewPanel("nodx.preview", "NODX Preview", vscode.ViewColumn.Beside, { enableScripts: false });
  panel.webview.html = previewShell(state.html);
}

async function openSemanticPreview(uri) {
  const target = uri ?? vscode.window.activeTextEditor?.document.uri;
  if (!target) return;
  const bytes = await vscode.workspace.fs.readFile(target);
  const parsed = await parseBytes(bytes);
  const doc = new NodxDocument(target, bytes, parsed);
  const state = await doc.viewState();
  const preview = await vscode.workspace.openTextDocument({ language: "markdown", content: state.semantic });
  await vscode.window.showTextDocument(preview, vscode.ViewColumn.Beside, true);
}

async function showInfoForActiveDocument(uri) {
  const target = uri ?? vscode.window.activeTextEditor?.document.uri;
  if (!target) return;
  const doc = new NodxDocument(target, await vscode.workspace.fs.readFile(target), await parseBytes(await vscode.workspace.fs.readFile(target)));
  const info = await doc.info();
  vscode.window.showInformationMessage(formatInfoSummary(info), { modal: true });
}

async function enableRemoteAssetsForActiveDocument(uri) {
  const editor = vscode.window.activeTextEditor;
  if (!editor || editor.document.languageId !== "nodx") {
    const target = uri ?? editor?.document.uri;
    if (!target) {
      vscode.window.showWarningMessage("Open a NODX source document to enable remote asset sources.");
      return;
    }
    const bytes = await vscode.workspace.fs.readFile(target);
    await vscode.workspace.fs.writeFile(target, textEncoder.encode(ensureRemoteAssetsFeature(textDecoder.decode(bytes))));
    return;
  }
  const fullRange = new vscode.Range(
    editor.document.positionAt(0),
    editor.document.positionAt(editor.document.getText().length),
  );
  await editor.edit((edit) => edit.replace(fullRange, ensureRemoteAssetsFeature(editor.document.getText())));
}

async function createPlainDocument() {
  const uri = await vscode.window.showSaveDialog({ filters: { NODX: ["nodx"] }, saveLabel: "Create plain NODX" });
  if (!uri) return;
  await vscode.workspace.fs.writeFile(uri, textEncoder.encode(plainTemplate()));
  await vscode.commands.executeCommand("vscode.openWith", uri, "default");
}

function ensureRemoteAssetsFeature(text) {
  if (/^\s*remote-assets:\s*true\s*$/m.test(text)) return text;
  if (/^(\s*)remote-assets:\s*false\s*$/m.test(text)) {
    return text.replace(/^(\s*)remote-assets:\s*false\s*$/m, "$1remote-assets: true");
  }
  if (!(text.startsWith("---\n") || text.startsWith("---\r\n"))) {
    return "---\nschema: nodx/1.0\nfeatures:\n  remote-assets: true\n---\n\n" + text;
  }
  const end = text.indexOf("\n---", 4);
  if (end < 0) return text;
  const head = text.slice(0, end);
  const tail = text.slice(end);
  const lines = head.split("\n");
  const featuresIndex = lines.findIndex((line) => line === "features:");
  if (featuresIndex >= 0) {
    lines.splice(featuresIndex + 1, 0, "  remote-assets: true");
  } else {
    lines.push("features:", "  remote-assets: true");
  }
  return lines.join("\n") + tail;
}

async function createPackageDocument() {
  const uri = await vscode.window.showSaveDialog({ filters: { NODX: ["nodx"] }, saveLabel: "Create NODX package" });
  if (!uri) return;
  const bytes = buildPackageBytes([
    { path: "content/document.nodx", kind: "content", text: plainTemplate() },
    { path: "themes/plain.nods", kind: "style", text: "body { color: #1f2937; }\n" },
  ], { components: [], themes: [{ path: "themes/plain.nods" }] }, "content/document.nodx");
  await vscode.workspace.fs.writeFile(uri, bytes);
  await vscode.commands.executeCommand("vscode.openWith", uri, NodxEditorProvider.viewType);
}

function completionProvider() {
  return {
    provideCompletionItems() {
      return [
        snippet("NODX front matter", "---\nschema: nodx/1.0\ntitle: ${1:Untitled}\ntheme: base\n---\n\n$0"),
        snippet("Section block", "::::section {#${1:id}}\n## ${2:Title}\n\n$0\n::::"),
        snippet("Note block", ":::note {type=\"${1:info}\"}\n$0\n:::"),
        snippet("Image", ":::image {src=\"${1:assets/image.svg}\" alt=\"${2:Description}\"}\n:::"),
        snippet("Table", ":::table {caption=\"${1:Caption}\"}\n:::row\n:::cell {header=\"true\" scope=\"col\"}\n${2:Header}\n:::\n:::\n:::"),
        snippet("Style block", ":::style\n${1:body { color: #1f2937; }}\n:::"),
      ];
    },
  };
}

function snippet(label, body) {
  const item = new vscode.CompletionItem(label, vscode.CompletionItemKind.Snippet);
  item.insertText = new vscode.SnippetString(body);
  return item;
}

function updateDiagnostics(uri, diagnostics) {
  diagnosticCollection.set(uri, diagnostics.map((diag) => {
    const line = Math.max(0, Number(diag.line ?? 1) - 1);
    const column = Math.max(0, Number(diag.column ?? 1) - 1);
    const severity = diag.severity === "warning" ? vscode.DiagnosticSeverity.Warning : vscode.DiagnosticSeverity.Error;
    const item = new vscode.Diagnostic(new vscode.Range(line, column, line, column + 1), `${diag.code}: ${diag.message}`, severity);
    item.source = "nodx";
    return item;
  }));
}

function diagnosticFromError(error) {
  return { code: "NODX-EDITOR", severity: "error", message: errorMessage(error), line: 1, column: 1, target: null };
}

function plainTemplate() {
  return "---\nschema: nodx/1.0\ntitle: Untitled NODX\ntheme: base\n---\n\n# Untitled NODX\n\nStart writing here.\n";
}

function frontMatterHeaders(text) {
  if (!text.startsWith("---")) return {};
  const end = text.indexOf("\n---", 3);
  if (end < 0) return {};
  const headers = {};
  for (const line of text.slice(3, end).split(/\r?\n/)) {
    const match = line.match(/^([A-Za-z0-9_-]+):\s*(.*)$/);
    if (match) headers[match[1]] = match[2];
  }
  return headers;
}

function formatInfoSummary(info) {
  const headers = Object.entries(info.headers).map(([key, value]) => `${key}: ${value}`).join("\n") || "No front matter headers.";
  const signature = info.signature.present
    ? `Signature: ${info.signature.verified ? "verified" : "not verified"} (${info.signature.reason})`
    : "Signature: not present";
  return `Representation: ${info.representation}\nEntry: ${info.entryPath}\n${signature}\n\n${headers}`;
}

function webviewHtml(webview) {
  const nonce = crypto.randomBytes(16).toString("hex");
  return `<!doctype html>
<html>
<head>
  <meta charset="utf-8">
  <meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; script-src 'nonce-${nonce}';">
  <style>
    body{margin:0;font:13px var(--vscode-font-family);color:var(--vscode-foreground);background:var(--vscode-editor-background)}
    .toolbar{height:36px;display:flex;align-items:center;gap:4px;padding:0 8px;border-bottom:1px solid var(--vscode-panel-border);background:var(--vscode-editorGroupHeader-tabsBackground);position:sticky;top:0;z-index:2}
    .toolgroup{display:flex;align-items:center;gap:2px;padding-right:6px;margin-right:4px;border-right:1px solid var(--vscode-panel-border)}
    button{font:inherit;color:var(--vscode-foreground);background:transparent;border:0;border-radius:3px;min-width:28px;height:28px;padding:0 7px}
    button:hover{background:var(--vscode-toolbar-hoverBackground)}
    button.active{color:var(--vscode-button-foreground);background:var(--vscode-button-background)}
    .badge{margin-left:auto;color:var(--vscode-descriptionForeground)}
    .layout{display:grid;grid-template-columns:280px minmax(0,1fr);min-height:calc(100vh - 37px)}
    .layout.plain{display:block}
    .layout.collapsed{grid-template-columns:0 minmax(0,1fr)}
    .tree{border-right:1px solid var(--vscode-panel-border);padding:8px;overflow:auto}
    .layout.collapsed .tree{display:none}
    .tree button{display:block;width:100%;text-align:left;margin:0 0 4px 0;color:var(--vscode-foreground);background:transparent;border-radius:3px}
    .tree button.active{background:var(--vscode-list-activeSelectionBackground);color:var(--vscode-list-activeSelectionForeground)}
    .main{min-width:0}
    .source-editor{position:relative;height:calc(100vh - 37px);background:var(--vscode-editor-background);overflow:hidden}
    .source-editor textarea,.source-highlight{box-sizing:border-box;position:absolute;inset:0;width:100%;height:100%;margin:0;border:0;padding:14px;font:13px/1.45 var(--vscode-editor-font-family);tab-size:2;white-space:pre-wrap;overflow:auto}
    .source-editor textarea{resize:none;outline:0;color:transparent;background:transparent;caret-color:var(--vscode-editor-foreground);z-index:1}
    .source-editor textarea::selection{background:var(--vscode-editor-selectionBackground)}
    .source-highlight{pointer-events:none;color:var(--vscode-editor-foreground)}
    .tok-fm{color:var(--vscode-symbolIcon-keywordForeground)}
    .tok-heading{color:var(--vscode-symbolIcon-functionForeground);font-weight:600}
    .tok-fence{color:var(--vscode-symbolIcon-operatorForeground)}
    .tok-attr{color:var(--vscode-symbolIcon-propertyForeground)}
    .tok-string{color:var(--vscode-symbolIcon-stringForeground)}
    .tok-code{color:var(--vscode-descriptionForeground)}
    iframe{width:100%;height:calc(100vh - 37px);border:0;background:white}
    pre{white-space:pre-wrap;margin:0;padding:14px;font:13px/1.45 var(--vscode-editor-font-family)}
    .diagnostics{padding:10px 14px}.diag{border-left:3px solid var(--vscode-errorForeground);padding:8px;margin:0 0 8px;background:var(--vscode-editorWidget-background)}
    dialog{border:1px solid var(--vscode-panel-border);background:var(--vscode-editorWidget-background);color:var(--vscode-foreground);max-width:760px;width:70vw}
    dialog pre{max-height:55vh;overflow:auto}
  </style>
</head>
<body>
  <div class="toolbar">
    <div class="toolgroup">
      <button id="preview" title="Preview" aria-label="Preview">&#128065;</button>
      <button id="source" title="Source" aria-label="Source">{ }</button>
      <button id="semantic" title="Semantic preview" aria-label="Semantic preview">&#9776;</button>
      <button id="diagnostics" title="Validate" aria-label="Validate">&#10003;</button>
    </div>
    <div class="toolgroup">
      <button id="toggleTree" title="Toggle package tree" aria-label="Toggle package tree">&#9776;</button>
      <button id="addAssetFile" title="Add local asset" aria-label="Add local asset">+</button>
      <button id="addAssetUrl" title="Add URL asset" aria-label="Add URL asset">&#8681;</button>
      <button id="remoteAssets" title="Enable remote asset sources" aria-label="Enable remote asset sources">&#128246;</button>
    </div>
    <button id="info" title="Info" aria-label="Info">i</button>
    <button id="save" title="Save" aria-label="Save">&#128190;</button>
    <span id="badge" class="badge"></span>
  </div>
  <div id="app"></div>
  <dialog id="modal"><form method="dialog"><button style="float:right">Close</button></form><pre id="modalText"></pre></dialog>
  <script nonce="${nonce}">
    const vscode = acquireVsCodeApi();
    let state = null;
    let currentPath = null;
    let treeCollapsed = false;
    const app = document.getElementById('app');
    const badge = document.getElementById('badge');
    for (const mode of ['preview','source','semantic','diagnostics']) {
      document.getElementById(mode).onclick = () => vscode.postMessage({type:'mode', mode});
    }
    document.getElementById('info').onclick = () => vscode.postMessage({type:'info'});
    document.getElementById('toggleTree').onclick = () => { treeCollapsed = !treeCollapsed; render(); };
    document.getElementById('addAssetFile').onclick = () => vscode.postMessage({type:'addAssetFile'});
    document.getElementById('addAssetUrl').onclick = () => vscode.postMessage({type:'addAssetUrl'});
    document.getElementById('remoteAssets').onclick = () => vscode.postMessage({type:'enableRemoteAssets'});
    document.getElementById('save').onclick = () => vscode.postMessage({type:'save'});
    window.addEventListener('message', (event) => {
      if (event.data.type === 'state') { state = event.data.state; render(); }
      if (event.data.type === 'info') showInfo(event.data.info);
      if (event.data.type === 'assetAdded') insertAssetSnippet(event.data.asset);
    });
    vscode.postMessage({type:'ready'});
    function render() {
      currentPath = currentPath || state.entryPath;
      badge.textContent = state.isPackage ? 'package: ' + state.entryPath : 'plain source';
      document.querySelectorAll('.toolbar button').forEach((button) => button.classList.remove('active'));
      const active = document.getElementById(state.mode);
      if (active) active.classList.add('active');
      document.getElementById('toggleTree').style.display = state.mode === 'source' && state.isPackage ? '' : 'none';
      if (state.mode === 'preview') app.innerHTML = '<iframe sandbox="" srcdoc="' + escapeAttr(state.html) + '"></iframe>';
      if (state.mode === 'semantic') app.innerHTML = '<pre>' + escapeHtml(state.semantic || '') + '</pre>';
      if (state.mode === 'diagnostics') renderDiagnostics();
      if (state.mode === 'source') renderSource();
    }
    function renderSource() {
      const files = state.files;
      const selected = files.find((file) => file.path === currentPath) || files[0];
      currentPath = selected.path;
      const assetRows = state.assets && state.assets.length ? '<hr>' + state.assets.map((asset) => '<button class="asset" data-snippet="' + escapeAttr(asset.path) + '">' + escapeHtml('asset · ' + asset.path) + '</button>').join('') : '';
      const tree = state.isPackage ? '<div class="tree">' + files.map((file) => '<button data-path="' + escapeAttr(file.path) + '" class="' + (file.path === currentPath ? 'active' : '') + '">' + escapeHtml(file.kind + ' · ' + file.path) + '</button>').join('') + assetRows + '</div>' : '';
      const layoutClass = state.isPackage ? (treeCollapsed ? 'layout collapsed' : 'layout') : 'layout plain';
      app.innerHTML = '<div class="' + layoutClass + '">' + tree + '<div class="main"><div class="source-editor"><pre class="source-highlight"></pre><textarea spellcheck="false"></textarea></div></div></div>';
      app.querySelectorAll('.tree button[data-path]').forEach((button) => button.onclick = () => { currentPath = button.dataset.path; renderSource(); });
      app.querySelectorAll('.tree button.asset').forEach((button) => button.onclick = () => insertText(button.dataset.snippet));
      const textarea = app.querySelector('textarea');
      const highlight = app.querySelector('.source-highlight');
      textarea.value = selected.text;
      updateHighlight(textarea, highlight);
      textarea.oninput = () => { updateHighlight(textarea, highlight); vscode.postMessage({type:'edit', path: currentPath, text: textarea.value}); };
      textarea.onscroll = () => { highlight.scrollTop = textarea.scrollTop; highlight.scrollLeft = textarea.scrollLeft; };
    }
    function insertAssetSnippet(asset) {
      if (!asset) return;
      setTimeout(() => insertText(asset.snippet || asset.path), 30);
    }
    function insertText(text) {
      const textarea = app.querySelector('textarea');
      if (!textarea) return;
      const start = textarea.selectionStart || 0;
      const end = textarea.selectionEnd || start;
      textarea.value = textarea.value.slice(0, start) + text + textarea.value.slice(end);
      textarea.selectionStart = textarea.selectionEnd = start + text.length;
      textarea.focus();
      const highlight = app.querySelector('.source-highlight');
      if (highlight) updateHighlight(textarea, highlight);
      vscode.postMessage({type:'edit', path: currentPath, text: textarea.value});
    }
    function updateHighlight(textarea, highlight) {
      highlight.innerHTML = highlightNodx(textarea.value);
      highlight.scrollTop = textarea.scrollTop;
      highlight.scrollLeft = textarea.scrollLeft;
    }
    function highlightNodx(text) {
      return escapeHtml(text).split('\\n').map((line) => {
        if (/^---\\s*$/.test(line) || /^[A-Za-z0-9_-]+:\\s*/.test(line)) return '<span class="tok-fm">' + line + '</span>';
        if (/^#{1,6}\\s/.test(line)) return '<span class="tok-heading">' + line + '</span>';
        if (/^:{3,}/.test(line)) return line.replace(/^(:{3,}\\w*)/, '<span class="tok-fence">$1</span>').replace(/\\{([^}]*)\\}/g, '<span class="tok-attr">{$1}</span>');
        if (/^\\x60\\x60\\x60/.test(line)) return '<span class="tok-code">' + line + '</span>';
        return line.replace(/(&quot;[^&]*?&quot;)/g, '<span class="tok-string">$1</span>').replace(/(\\{#[^}]+\\})/g, '<span class="tok-attr">$1</span>');
      }).join('\\n');
    }
    function renderDiagnostics() {
      const rows = state.diagnostics.length ? state.diagnostics.map((diag) => '<div class="diag"><strong>' + escapeHtml(diag.code || 'NODX') + '</strong> ' + escapeHtml(diag.severity || 'error') + '<br>' + escapeHtml(diag.message || '') + '<br><small>' + escapeHtml(String(diag.target || 'line ' + (diag.line || 1))) + '</small></div>').join('') : '<p>No diagnostics.</p>';
      app.innerHTML = '<div class="diagnostics">' + rows + '</div>';
    }
    function showInfo(info) {
      document.getElementById('modalText').textContent = JSON.stringify(info, null, 2);
      document.getElementById('modal').showModal();
    }
    function escapeHtml(text) { return String(text).replace(/[&<>]/g, (ch) => ({'&':'&amp;','<':'&lt;','>':'&gt;'}[ch])); }
    function escapeAttr(text) { return escapeHtml(text).replace(/"/g, '&quot;'); }
  </script>
</body>
</html>`;
}

function previewShell(html) {
  return html;
}

function errorHtml(error) {
  return `<!doctype html><meta charset="utf-8"><body><pre>${escapeHtml(errorMessage(error))}</pre></body>`;
}

function escapeHtml(text) {
  return String(text).replace(/[&<>]/g, (ch) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;" }[ch]));
}

function errorMessage(error) {
  return error && error.message ? error.message : String(error);
}

module.exports = { activate, deactivate };

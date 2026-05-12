import { parse } from "./blockParser.mjs";

const textDecoder = new TextDecoder("utf-8", { fatal: true });

export function isPackagedNodx(bytes) {
  return bytes.length >= 4 && bytes[0] === 0x50 && bytes[1] === 0x4b && bytes[2] === 0x03 && bytes[3] === 0x04;
}

export function openStoredPackage(bytes) {
  if (!(bytes instanceof Uint8Array)) bytes = new Uint8Array(bytes);
  if (!isPackagedNodx(bytes)) throw new Error("not a packaged NODX");
  const eocd = findEocd(bytes);
  if (eocd < 0) throw new Error("ZIP end of central directory not found");
  if (u16(bytes, eocd + 4) !== 0 || u16(bytes, eocd + 6) !== 0) throw new Error("multi-disk ZIP is not supported");
  const count = u16(bytes, eocd + 10);
  const cdOffset = u32(bytes, eocd + 16);
  const files = new Map();
  let pos = cdOffset;
  for (let i = 0; i < count; i++) {
    if (u32(bytes, pos) !== 0x02014b50) throw new Error("invalid ZIP central directory");
    const flags = u16(bytes, pos + 8);
    const compression = u16(bytes, pos + 10);
    const compressedSize = u32(bytes, pos + 20);
    const uncompressedSize = u32(bytes, pos + 24);
    const nameLen = u16(bytes, pos + 28);
    const extraLen = u16(bytes, pos + 30);
    const commentLen = u16(bytes, pos + 32);
    const localOffset = u32(bytes, pos + 42);
    if (flags & 1) throw new Error("encrypted ZIP entries are not supported");
    if (flags & 8) throw new Error("ZIP data descriptors are not supported");
    if (compression !== 0) throw new Error("browser package reader supports stored ZIP entries only");
    const name = decode(bytes.subarray(pos + 46, pos + 46 + nameLen));
    if (!isSafePackagePath(name)) throw new Error("unsafe package path: " + name);
    files.set(name, readStoredEntry(bytes, localOffset, compressedSize, uncompressedSize, name));
    pos += 46 + nameLen + extraLen + commentLen;
  }
  const mimetype = files.get("mimetype");
  if (!mimetype || decode(mimetype) !== "application/nodx+zip") throw new Error("invalid NODX package mimetype");
  const manifestBytes = files.get("manifest.yaml");
  if (!manifestBytes) throw new Error("package is missing manifest.yaml");
  const manifest = parseManifest(decode(manifestBytes));
  if (manifest.schema !== "nodx-package/1.0") throw new Error("package manifest schema must be nodx-package/1.0");
  if (!manifest.entry) throw new Error("package manifest is missing entry");
  if (!files.has(manifest.entry)) throw new Error("package entry document is missing");
  return { entryPath: manifest.entry, entryBytes: files.get(manifest.entry), files, manifest };
}

export function packageEntryText(bytes) {
  return decode(openStoredPackage(bytes).entryBytes);
}

export function parsePackagedDocument(bytes, options = {}) {
  const pkg = openStoredPackage(bytes);
  const doc = parse(decode(pkg.entryBytes));
  return applyPackageExtensions(doc, pkg, options);
}

export function applyPackageExtensions(doc, pkg, options = {}) {
  const out = {
    ...doc,
    meta: {
      ...doc.meta,
      components: [...(Array.isArray(doc.meta.components) ? doc.meta.components : [])],
      stylesheets: [...(Array.isArray(doc.meta.stylesheets) ? doc.meta.stylesheets : [])],
    },
  };
  for (const component of packageComponentDefinitions(pkg, options)) out.meta.components.push(component);
  for (const css of packageThemeStylesheets(pkg, options)) out.meta.stylesheets.push(css);
  if (!out.meta.components.length) delete out.meta.components;
  if (!out.meta.stylesheets.length) delete out.meta.stylesheets;
  return out;
}

export function packageComponentDefinitions(pkg, options = {}) {
  const files = pkg.files;
  const paths = manifestPaths(pkg.manifest.components);
  if (options.autodiscoverComponents) {
    for (const path of files.keys()) {
      if (path.startsWith("components/") && path.endsWith(".nodx") && !paths.includes(path)) paths.push(path);
    }
  }
  return paths.map((path) => parseComponentFile(decodeRequired(files, path), path));
}

export function packageThemeStylesheets(pkg, options = {}) {
  const files = pkg.files;
  const paths = manifestPaths(pkg.manifest.themes);
  if (options.autodiscoverThemes) {
    for (const path of files.keys()) {
      if ((path.startsWith("themes/") || path.startsWith("styles/")) && path.endsWith(".nods") && !paths.includes(path)) paths.push(path);
    }
  }
  return paths.map((path) => decodeRequired(files, path));
}

function parseComponentFile(text, path) {
  const body = stripFrontMatter(text);
  const parsed = parse(text);
  const name = typeof parsed.meta.name === "string" ? parsed.meta.name : basename(path).replace(/\.nodx$/, "");
  const component = { name, template: body };
  if (typeof parsed.meta.style === "string" && parsed.meta.style.trim()) component.style = parsed.meta.style;
  return component;
}

function stripFrontMatter(text) {
  if (!text.startsWith("---\n")) return text;
  const end = text.indexOf("\n---", 4);
  if (end < 0) return text;
  let bodyStart = end + 4;
  if (text[bodyStart] === "\r") bodyStart += 1;
  if (text[bodyStart] === "\n") bodyStart += 1;
  return text.slice(bodyStart);
}

function readStoredEntry(bytes, offset, compressedSize, uncompressedSize, expectedName) {
  if (u32(bytes, offset) !== 0x04034b50) throw new Error("invalid ZIP local header");
  if (u16(bytes, offset + 8) !== 0) throw new Error("browser package reader supports stored ZIP entries only");
  if (u32(bytes, offset + 18) !== compressedSize || u32(bytes, offset + 22) !== uncompressedSize) {
    throw new Error("ZIP local header does not match central directory");
  }
  const nameLen = u16(bytes, offset + 26);
  const extraLen = u16(bytes, offset + 28);
  const name = decode(bytes.subarray(offset + 30, offset + 30 + nameLen));
  if (name !== expectedName) throw new Error("ZIP local header name mismatch");
  const start = offset + 30 + nameLen + extraLen;
  const end = start + compressedSize;
  if (end > bytes.length) throw new Error("ZIP entry data is out of bounds");
  return bytes.slice(start, end);
}

function parseManifest(text) {
  const manifest = { schema: "", entry: "", signature: "", entries: [], components: [], themes: [] };
  let inEntries = false;
  let listField = null;
  for (const raw of text.split(/\r?\n/)) {
    const line = raw.trimEnd();
    if (!line.trim() || line.trimStart().startsWith("#")) continue;
    if (!raw.startsWith(" ") && !raw.startsWith("-")) {
      inEntries = false;
      listField = null;
    }
    if (line.startsWith("schema:")) manifest.schema = unquote(line.slice(7).trim());
    else if (line.startsWith("entry:")) manifest.entry = unquote(line.slice(6).trim());
    else if (line.startsWith("signature:")) manifest.signature = unquote(line.slice(10).trim());
    else if (line === "entries:") inEntries = true;
    else if (line === "components:") listField = "components";
    else if (line === "themes:") listField = "themes";
    else if (inEntries && line.trimStart().startsWith("- path:")) {
      manifest.entries.push({ path: unquote(line.trimStart().slice(7).trim()) });
    } else if (listField && line.trimStart().startsWith("- path:")) {
      manifest[listField].push({ path: unquote(line.trimStart().slice(7).trim()) });
    }
  }
  return manifest;
}

function manifestPaths(items) {
  return Array.isArray(items) ? items.map((item) => typeof item === "string" ? item : item?.path).filter(Boolean) : [];
}

function decodeRequired(files, path) {
  const bytes = files.get(path);
  if (!bytes) throw new Error("package manifest references missing path: " + path);
  return decode(bytes);
}

function basename(path) {
  return path.split("/").pop() ?? path;
}

function unquote(value) {
  if ((value.startsWith('"') && value.endsWith('"')) || (value.startsWith("'") && value.endsWith("'"))) {
    return value.slice(1, -1);
  }
  return value;
}

function findEocd(bytes) {
  for (let i = bytes.length - 22; i >= Math.max(0, bytes.length - 65557); i--) {
    if (u32(bytes, i) === 0x06054b50) return i;
  }
  return -1;
}

function isSafePackagePath(path) {
  if (!path || path.startsWith("/") || path.includes("\\") || path.includes("//")) return false;
  return path.split("/").every((part) => part && part !== "." && part !== "..");
}

function decode(bytes) {
  return textDecoder.decode(bytes);
}

function u16(bytes, pos) {
  return bytes[pos] | (bytes[pos + 1] << 8);
}

function u32(bytes, pos) {
  return (bytes[pos] | (bytes[pos + 1] << 8) | (bytes[pos + 2] << 16) | (bytes[pos + 3] << 24)) >>> 0;
}

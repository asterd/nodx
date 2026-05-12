import { parse } from "./blockParser.mjs";
import { sha256Base64Url } from "./bytes.mjs";
import { DEFAULT_LIMITS } from "./limits.mjs";
import { normalizePackagePath } from "./url.mjs";

const textDecoder = new TextDecoder("utf-8", { fatal: true });

export function isPackagedNodx(bytes) {
  return bytes.length >= 4 && bytes[0] === 0x50 && bytes[1] === 0x4b && bytes[2] === 0x03 && bytes[3] === 0x04;
}

export function openStoredPackage(bytes, limits = DEFAULT_LIMITS) {
  if (!(bytes instanceof Uint8Array)) bytes = new Uint8Array(bytes);
  if (!isPackagedNodx(bytes)) throw new Error("not a packaged NODX");
  const entries = zipEntries(bytes, limits);
  if (entries.length > limits.packageFileCount) throw new Error("package file count limit exceeded");
  const total = entries.reduce((sum, entry) => sum + entry.uncompressedSize, 0);
  if (total > limits.packageUncompressedBytes) throw new Error("package uncompressed size limit exceeded");
  if (entries[0]?.name !== "mimetype" || entries[0].compression !== 0) throw new Error("first ZIP entry must be mimetype");

  const files = new Map();
  for (const entry of entries) files.set(entry.name, readStoredEntry(bytes, entry, limits));

  const mimetype = files.get("mimetype");
  if (!mimetype || decode(mimetype) !== "application/nodx+zip") throw new Error("invalid NODX package mimetype");
  const manifestBytes = files.get("manifest.yaml");
  if (!manifestBytes) throw new Error("package is missing manifest.yaml");
  const manifest = parseManifest(decode(manifestBytes), limits);
  if (manifest.schema !== "nodx-package/1.0") throw new Error("package manifest schema must be nodx-package/1.0");
  if (manifest.entries.length > limits.manifestEntries) throw new Error("package manifest entry limit exceeded");
  for (const item of manifest.entries) {
    const data = files.get(item.path);
    if (!data) throw new Error("manifest lists a missing package entry");
    if (item.size !== null && item.size !== data.length) throw new Error("package manifest size does not match entry bytes");
    if (item.sha256 && item.sha256 !== sha256Base64Url(data)) throw new Error("package digest mismatch");
  }
  for (const path of [...manifestPaths(manifest.components), ...manifestPaths(manifest.themes)]) {
    if (!files.has(path)) throw new Error("manifest lists a missing package extension");
    if (!manifest.entries.some((entry) => entry.path === path)) throw new Error("package extension paths must also be listed in manifest entries");
  }
  if (!manifest.entry) throw new Error("package manifest is missing entry");
  manifest.entry = safePackagePath(manifest.entry, limits);
  if (!files.has(manifest.entry)) throw new Error("package entry document is missing");
  if (manifest.signature) {
    manifest.signature = safePackagePath(manifest.signature, limits);
    if (!files.has(manifest.signature)) throw new Error("package manifest references a missing signature");
  }
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

function zipEntries(bytes, limits) {
  const eocd = findEocd(bytes);
  if (eocd < 0) throw new Error("ZIP end of central directory not found");
  if (u16(bytes, eocd + 4) !== 0 || u16(bytes, eocd + 6) !== 0) throw new Error("multi-disk ZIP is not supported");
  const count = u16(bytes, eocd + 10);
  const cdSize = u32(bytes, eocd + 12);
  const cdOffset = u32(bytes, eocd + 16);
  if (count === 0xffff || cdSize === 0xffffffff || cdOffset === 0xffffffff) throw new Error("ZIP64 packages are not supported");

  let pos = cdOffset;
  const entries = [];
  const seen = new Set();
  for (let i = 0; i < count; i += 1) {
    if (u32(bytes, pos) !== 0x02014b50) throw new Error("invalid ZIP central directory");
    const flags = u16(bytes, pos + 8);
    const compression = u16(bytes, pos + 10);
    const crc32 = u32(bytes, pos + 16);
    const compressedSize = u32(bytes, pos + 20);
    const uncompressedSize = u32(bytes, pos + 24);
    const nameLen = u16(bytes, pos + 28);
    const extraLen = u16(bytes, pos + 30);
    const commentLen = u16(bytes, pos + 32);
    const externalAttrs = u32(bytes, pos + 38);
    const localOffset = u32(bytes, pos + 42);
    if (compressedSize === 0xffffffff || uncompressedSize === 0xffffffff || localOffset === 0xffffffff) throw new Error("ZIP64 packages are not supported");
    if (uncompressedSize > limits.packageEntryBytes) throw new Error("package entry size limit exceeded");
    if (compressedSize === 0 && uncompressedSize > 0) throw new Error("package compression ratio limit exceeded");
    if (compressedSize > 0 && uncompressedSize > compressedSize * limits.packageCompressionRatio) throw new Error("package compression ratio limit exceeded");
    if (flags & 1) throw new Error("encrypted ZIP entries are not supported");
    if (flags & 8) throw new Error("ZIP data descriptors are not supported");
    if (compression !== 0) throw new Error("browser package reader supports stored ZIP entries only");
    rejectSpecialFile(externalAttrs);
    const nameStart = pos + 46;
    const nameEnd = nameStart + nameLen;
    const extraEnd = nameEnd + extraLen;
    if (extraEnd > bytes.length) throw new Error("ZIP entry name is out of bounds");
    rejectZip64Extra(bytes.subarray(nameEnd, extraEnd));
    const name = safePackagePath(decode(bytes.subarray(nameStart, nameEnd)), limits);
    if (seen.has(name)) throw new Error("duplicate package entry path");
    seen.add(name);
    entries.push({ name, compression, crc32, compressedSize, uncompressedSize, localOffset, externalAttrs });
    pos = extraEnd + commentLen;
  }
  return entries.sort((a, b) => a.localOffset - b.localOffset);
}

function readStoredEntry(bytes, entry, limits) {
  const offset = entry.localOffset;
  if (u32(bytes, offset) !== 0x04034b50) throw new Error("invalid ZIP local header");
  const flags = u16(bytes, offset + 6);
  const compression = u16(bytes, offset + 8);
  const crc = u32(bytes, offset + 14);
  const compressedSize = u32(bytes, offset + 18);
  const uncompressedSize = u32(bytes, offset + 22);
  if (flags & 1) throw new Error("encrypted ZIP entries are not supported");
  if (flags & 8) throw new Error("ZIP data descriptors are not supported");
  if (compression !== entry.compression || crc !== entry.crc32 || compressedSize !== entry.compressedSize || uncompressedSize !== entry.uncompressedSize) {
    throw new Error("ZIP local header does not match central directory");
  }
  const nameLen = u16(bytes, offset + 26);
  const extraLen = u16(bytes, offset + 28);
  const nameStart = offset + 30;
  const nameEnd = nameStart + nameLen;
  const extraEnd = nameEnd + extraLen;
  if (extraEnd > bytes.length) throw new Error("ZIP entry data is out of bounds");
  rejectZip64Extra(bytes.subarray(nameEnd, extraEnd));
  const name = safePackagePath(decode(bytes.subarray(nameStart, nameEnd)), limits);
  if (name !== entry.name) throw new Error("ZIP local header name mismatch");
  rejectSpecialFile(entry.externalAttrs);
  const end = extraEnd + entry.compressedSize;
  if (end > bytes.length) throw new Error("ZIP entry data is out of bounds");
  const data = bytes.slice(extraEnd, end);
  if (data.length !== entry.uncompressedSize) throw new Error("stored ZIP entry has inconsistent sizes");
  if (crc32(data) !== entry.crc32) throw new Error("ZIP CRC mismatch");
  if (looksLikeZip(data)) throw new Error("nested ZIP archives are not supported");
  return data;
}

function parseManifest(text, limits) {
  const manifest = { schema: "", entry: "", signature: "", entries: [], components: [], themes: [] };
  let section = "";
  let currentEntry = null;
  const seenTop = new Set();
  for (const raw of text.split(/\r?\n/)) {
    const line = raw.trimEnd();
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;
    if (!raw.startsWith(" ") && !raw.startsWith("-")) {
      section = "";
      currentEntry = null;
      const key = trimmed.endsWith(":") ? trimmed.slice(0, -1) : trimmed.split(":", 1)[0];
      if (seenTop.has(key)) throw new Error("duplicate manifest key");
      seenTop.add(key);
    }
    if (line.startsWith("schema:")) manifest.schema = unquote(line.slice(7).trim());
    else if (line.startsWith("entry:")) manifest.entry = unquote(line.slice(6).trim());
    else if (line.startsWith("signature:")) manifest.signature = unquote(line.slice(10).trim());
    else if (line === "entries:" || line === "components:" || line === "themes:") section = line.slice(0, -1);
    else if (section === "entries" && trimmed.startsWith("- path:")) {
      currentEntry = { path: safePackagePath(unquote(trimmed.slice(7).trim()), limits), size: null, sha256: "" };
      manifest.entries.push(currentEntry);
    } else if (section === "entries" && currentEntry && trimmed.startsWith("size:")) {
      currentEntry.size = Number(trimmed.slice(5).trim());
      if (!Number.isSafeInteger(currentEntry.size) || currentEntry.size < 0) throw new Error("invalid manifest entry size");
    } else if (section === "entries" && currentEntry && trimmed.startsWith("sha256:")) {
      currentEntry.sha256 = unquote(trimmed.slice(7).trim());
    } else if ((section === "components" || section === "themes") && trimmed.startsWith("- path:")) {
      manifest[section].push({ path: safePackagePath(unquote(trimmed.slice(7).trim()), limits) });
    }
  }
  return manifest;
}

function manifestPaths(items) {
  return Array.isArray(items) ? items.map((item) => typeof item === "string" ? item : item?.path).filter(Boolean) : [];
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

function safePackagePath(path, limits) {
  const normalized = normalizePackagePath(path, limits);
  if (normalized === null) throw new Error("unsafe package path: " + path);
  return normalized;
}

function rejectZip64Extra(extra) {
  let pos = 0;
  while (pos + 4 <= extra.length) {
    const header = u16(extra, pos);
    const len = u16(extra, pos + 2);
    pos += 4;
    const end = pos + len;
    if (end > extra.length) throw new Error("invalid ZIP extra field");
    if (header === 0x0001) throw new Error("ZIP64 packages are not supported");
    pos = end;
  }
  if (pos !== extra.length) throw new Error("invalid ZIP extra field");
}

function rejectSpecialFile(externalAttrs) {
  const mode = externalAttrs >>> 16;
  if (mode === 0) return;
  if ((mode & 0o170000) !== 0o100000) throw new Error("ZIP special files are not supported");
}

function looksLikeZip(data) {
  return data.length >= 4 && data[0] === 0x50 && data[1] === 0x4b && (data[2] === 0x03 || data[2] === 0x05 || data[2] === 0x07);
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
  if ((value.startsWith('"') && value.endsWith('"')) || (value.startsWith("'") && value.endsWith("'"))) return value.slice(1, -1);
  return value;
}

function findEocd(bytes) {
  for (let i = bytes.length - 22; i >= Math.max(0, bytes.length - 65557); i -= 1) {
    if (u32(bytes, i) === 0x06054b50) return i;
  }
  return -1;
}

function decode(bytes) {
  return textDecoder.decode(bytes);
}

function u16(bytes, pos) {
  if (pos + 2 > bytes.length) throw new Error("unexpected end of ZIP data");
  return bytes[pos] | (bytes[pos + 1] << 8);
}

function u32(bytes, pos) {
  if (pos + 4 > bytes.length) throw new Error("unexpected end of ZIP data");
  return (bytes[pos] | (bytes[pos + 1] << 8) | (bytes[pos + 2] << 16) | (bytes[pos + 3] << 24)) >>> 0;
}

function crc32(bytes) {
  let crc = 0xffffffff;
  for (const byte of bytes) crc = (crc >>> 8) ^ CRC_TABLE[(crc ^ byte) & 0xff];
  return (crc ^ 0xffffffff) >>> 0;
}

const CRC_TABLE = new Uint32Array(256);
for (let i = 0; i < 256; i += 1) {
  let c = i;
  for (let k = 0; k < 8; k += 1) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  CRC_TABLE[i] = c >>> 0;
}

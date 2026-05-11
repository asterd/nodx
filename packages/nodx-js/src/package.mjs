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
  const manifest = { schema: "", entry: "", signature: "", entries: [] };
  let inEntries = false;
  for (const raw of text.split(/\r?\n/)) {
    const line = raw.trimEnd();
    if (!line.trim() || line.trimStart().startsWith("#")) continue;
    if (!raw.startsWith(" ") && !raw.startsWith("-")) inEntries = false;
    if (line.startsWith("schema:")) manifest.schema = unquote(line.slice(7).trim());
    else if (line.startsWith("entry:")) manifest.entry = unquote(line.slice(6).trim());
    else if (line.startsWith("signature:")) manifest.signature = unquote(line.slice(10).trim());
    else if (line === "entries:") inEntries = true;
    else if (inEntries && line.trimStart().startsWith("- path:")) {
      manifest.entries.push({ path: unquote(line.trimStart().slice(7).trim()) });
    }
  }
  return manifest;
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

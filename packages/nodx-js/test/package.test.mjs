import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

import { sha256Base64Url } from "../src/bytes.mjs";
import { openStoredPackage, packageEntryText, parse } from "../parser.mjs";

test("opens stored packaged NODX example", () => {
  const bytes = readFileSync(new URL("../../../examples/extended-showcase-bundled.nodx", import.meta.url));
  const pkg = openStoredPackage(bytes);
  assert.equal(pkg.entryPath, "content/document.nodx");
  assert.ok(pkg.files.has("assets/reference-pipeline.svg"));
  assert.match(packageEntryText(bytes), /Extended Showcase/);
  const doc = parse(packageEntryText(bytes));
  assert.equal(doc.schema, "nodx/1.0");
});

test("rejects package manifest digest mismatch", () => {
  const zip = buildZip([
    ["mimetype", enc("application/nodx+zip")],
    ["manifest.yaml", enc(manifest("doc.nodx", enc("# A\n"), "sha256-bad"))],
    ["doc.nodx", enc("# A\n")],
  ]);
  assert.throws(() => openStoredPackage(zip), /digest mismatch/);
});

test("rejects percent-encoded package traversal", () => {
  const zip = buildZip([
    ["mimetype", enc("application/nodx+zip")],
    ["manifest.yaml", enc("schema: nodx-package/1.0\nentry: a/%2e%2e/doc.nodx\n")],
    ["a/%2e%2e/doc.nodx", enc("# A\n")],
  ]);
  assert.throws(() => openStoredPackage(zip), /unsafe package path/);
});

test("rejects duplicate package entry paths", () => {
  const zip = buildZip([
    ["mimetype", enc("application/nodx+zip")],
    ["manifest.yaml", enc(manifest("doc.nodx", enc("# A\n")))],
    ["doc.nodx", enc("# A\n")],
    ["doc.nodx", enc("# B\n")],
  ]);
  assert.throws(() => openStoredPackage(zip), /duplicate package entry path/);
});

function manifest(path, data, digest = sha256Base64Url(data)) {
  return `schema: nodx-package/1.0
entry: ${path}
entries:
  - path: ${path}
    size: ${data.length}
    sha256: ${digest}
`;
}

function buildZip(entries) {
  const chunks = [];
  const central = [];
  let offset = 0;
  for (const [name, data] of entries) {
    const nameBytes = enc(name);
    const crc = crc32(data);
    const local = concat(
      u32(0x04034b50), u16(20), u16(0), u16(0), u16(0), u16(0),
      u32(crc), u32(data.length), u32(data.length), u16(nameBytes.length), u16(0),
      nameBytes, data,
    );
    chunks.push(local);
    central.push({ nameBytes, data, crc, offset });
    offset += local.length;
  }
  const cdOffset = offset;
  for (const entry of central) {
    const header = concat(
      u32(0x02014b50), u16(20), u16(20), u16(0), u16(0), u16(0), u16(0),
      u32(entry.crc), u32(entry.data.length), u32(entry.data.length),
      u16(entry.nameBytes.length), u16(0), u16(0), u16(0), u16(0), u32(0o100644 << 16), u32(entry.offset),
      entry.nameBytes,
    );
    chunks.push(header);
    offset += header.length;
  }
  const cdSize = offset - cdOffset;
  chunks.push(concat(u32(0x06054b50), u16(0), u16(0), u16(central.length), u16(central.length), u32(cdSize), u32(cdOffset), u16(0)));
  return concat(...chunks);
}

function enc(text) {
  return new TextEncoder().encode(text);
}

function concat(...parts) {
  const out = new Uint8Array(parts.reduce((sum, part) => sum + part.length, 0));
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

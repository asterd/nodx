import { DEFAULT_LIMITS } from "./limits.mjs";

// JavaScript port of nodx-url::ResourcePolicy. Mirrors the Rust reference byte
// for byte: the conformance gate diffs accept/deny decisions through the shared
// `spec/tests/security/url-policy.tsv` corpus.

export const ReferenceKind = Object.freeze({
  Link: "link",
  Asset: "asset",
  Style: "style",
  Include: "include",
  Font: "font",
  MediaFallback: "media-fallback",
});

const FORBIDDEN_SCHEMES = new Set([
  "javascript",
  "vbscript",
  "file",
  "jar",
  "chrome",
  "about",
]);

const ALLOWED_DATA_MIMES = new Set([
  "image/png",
  "image/jpeg",
  "image/webp",
  "image/gif",
]);

export function classifyUri(kind, raw, limits = DEFAULT_LIMITS) {
  const trimmed = trimAsciiWhitespace(raw);
  if (trimmed.length === 0) return { ok: false, error: "empty" };
  if (trimmed.length > limits.urlBytes) return { ok: false, error: "too-long" };
  const ctrl = rejectControlOrBackslash(trimmed);
  if (ctrl !== null) return { ok: false, error: ctrl };

  if (trimmed.startsWith("#")) {
    if (kind === ReferenceKind.Link) {
      return { ok: true, class: { type: "fragment", value: trimmed } };
    }
    return { ok: false, error: "invalid-package-path" };
  }

  const scheme = schemePrefix(trimmed);
  if (scheme === "error") return { ok: false, error: "unsafe-scheme" };
  if (scheme !== null) return classifyScheme(kind, trimmed, scheme, limits);

  const path = normalizePackagePath(trimmed, limits);
  if (path === null) return { ok: false, error: "invalid-package-path" };
  return { ok: true, class: { type: "package-relative", value: path } };
}

export function normalizePackagePath(raw, limits = DEFAULT_LIMITS) {
  const trimmed = trimAsciiWhitespace(raw);
  if (trimmed.length === 0) return null;
  if (trimmed.length > limits.urlBytes) return null;
  if (rejectControlOrBackslash(trimmed) !== null) return null;
  if (trimmed.startsWith("/")) return null;
  if (schemePrefix(trimmed) !== null) return null;
  if (trimmed.length > limits.packagePathBytes) return null;
  const parts = trimmed.split("/");
  if (parts.length > limits.packagePathSegments) return null;
  const normalized = [];
  for (const part of parts) {
    if (part === "" || part === ".") return null;
    const decoded = percentDecodeAscii(part);
    if (decoded === null) return null;
    if (part === ".." || decoded === "..") return null;
    if (
      decoded.includes(":") ||
      decoded.includes("/") ||
      decoded.includes("\\") ||
      hasControlChar(decoded)
    ) {
      return null;
    }
    normalized.push(part);
  }
  return normalized.join("/");
}

function classifyScheme(kind, raw, scheme, limits) {
  if (FORBIDDEN_SCHEMES.has(scheme)) return { ok: false, error: "unsafe-scheme" };
  if (
    kind === ReferenceKind.Link &&
    ["http", "https", "mailto", "tel"].includes(scheme)
  ) {
    return { ok: true, class: { type: "absolute", value: raw, scheme } };
  }
  if (kind === ReferenceKind.Asset && scheme === "data") {
    const mime = validateDataUri(raw, limits);
    if (mime === null) return { ok: false, error: "unsafe-data" };
    return { ok: true, class: { type: "data", value: raw, mime } };
  }
  return { ok: false, error: "unsafe-scheme" };
}

function rejectControlOrBackslash(input) {
  if (input.includes("\\")) return "backslash";
  if (hasControlChar(input)) return "control";
  return null;
}

function hasControlChar(input) {
  for (let i = 0; i < input.length; i += 1) {
    const code = input.charCodeAt(i);
    if (code < 0x20 || code === 0x7f) return true;
  }
  return false;
}

function schemePrefix(input) {
  let boundary = input.length;
  for (let i = 0; i < input.length; i += 1) {
    const c = input[i];
    if (c === ":" || c === "/" || c === "?" || c === "#") {
      boundary = i;
      break;
    }
  }
  if (input[boundary] !== ":") return null;
  if (boundary === 0) return "error";
  const decoded = percentDecodeAscii(input.slice(0, boundary));
  if (decoded === null) return "error";
  const lower = decoded.toLowerCase();
  if (lower.length === 0) return "error";
  for (let i = 0; i < lower.length; i += 1) {
    const ch = lower[i];
    if (!isSchemeChar(ch)) return "error";
  }
  return lower;
}

function isSchemeChar(ch) {
  return (
    (ch >= "a" && ch <= "z") ||
    (ch >= "0" && ch <= "9") ||
    ch === "+" ||
    ch === "-" ||
    ch === "."
  );
}

function validateDataUri(raw, limits) {
  if (raw.length > limits.dataUriBytes) return null;
  const colon = raw.indexOf(":");
  if (colon < 0) return null;
  if (raw.slice(0, colon).toLowerCase() !== "data") return null;
  const rest = raw.slice(colon + 1);
  const comma = rest.indexOf(",");
  if (comma < 0) return null;
  const meta = rest.slice(0, comma);
  const mime = (meta.split(";")[0] ?? "").trim().toLowerCase();
  return ALLOWED_DATA_MIMES.has(mime) ? mime : null;
}

function percentDecodeAscii(input) {
  let out = "";
  let i = 0;
  while (i < input.length) {
    if (input[i] === "%") {
      if (i + 2 >= input.length) return null;
      const hi = hexValue(input.charCodeAt(i + 1));
      const lo = hexValue(input.charCodeAt(i + 2));
      if (hi === null || lo === null) return null;
      const v = (hi << 4) | lo;
      if (v >= 0x80) return null;
      out += String.fromCharCode(v);
      i += 3;
    } else {
      out += input[i];
      i += 1;
    }
  }
  return out;
}

function hexValue(code) {
  if (code >= 0x30 && code <= 0x39) return code - 0x30;
  if (code >= 0x61 && code <= 0x66) return code - 0x61 + 10;
  if (code >= 0x41 && code <= 0x46) return code - 0x41 + 10;
  return null;
}

function trimAsciiWhitespace(input) {
  let start = 0;
  let end = input.length;
  while (start < end && isAsciiWhitespace(input.charCodeAt(start))) start += 1;
  while (end > start && isAsciiWhitespace(input.charCodeAt(end - 1))) end -= 1;
  return input.slice(start, end);
}

function isAsciiWhitespace(code) {
  return code === 0x09 || code === 0x0a || code === 0x0c || code === 0x0d || code === 0x20;
}

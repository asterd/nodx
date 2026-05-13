import { emptyAttrs } from "./ast.mjs";

export function parseAttrs(raw) {
  const attrs = emptyAttrs();
  const s = raw.trim();
  if (!s.startsWith("{") || !s.endsWith("}")) return attrs;
  for (const token of splitAttrs(s.slice(1, -1))) {
    if (token.startsWith("#")) attrs.id = token.slice(1);
    else if (token.startsWith(".")) attrs.classes.push(token.slice(1));
    else {
      const i = token.indexOf("=");
      if (i > 0) applyAttr(attrs, token.slice(0, i), unquote(token.slice(i + 1)));
      else if (token === "highlight") {
        attrs.styles["background-color"] = "color-mix(in srgb, var(--nodx-color-accent) 14%, transparent)";
        attrs.styles.padding = "0.05em 0.25em";
        attrs.styles["border-radius"] = "0.2em";
      }
    }
  }
  attrs.classes = [...new Set(attrs.classes)].sort();
  if (!Object.keys(attrs.styles).length) delete attrs.styles;
  return attrs;
}

export function mergeClassSuffix(attrs, suffix) {
  for (const cls of parseClassSuffix(suffix)) attrs.classes.push(cls);
  attrs.classes = [...new Set(attrs.classes)].sort();
  return attrs;
}

export function parseClassSuffix(input) {
  const classes = [];
  let i = 0;
  while (input[i] === ".") {
    const match = input.slice(i + 1).match(/^[A-Za-z_][A-Za-z0-9_-]*/);
    if (!match) break;
    classes.push(match[0]);
    i += match[0].length + 1;
  }
  return classes;
}

function applyAttr(attrs, key, value) {
  if (key === "class") {
    attrs.classes.push(...value.split(/\s+/).filter(Boolean));
    return;
  }
  const prop = STYLE_SHORTHANDS[key] ?? (ALLOWED_INLINE_PROPERTIES.has(key) ? key : null);
  if (prop && safeInlineStyleValue(value)) {
    attrs.styles[prop] = value;
    return;
  }
  attrs.attrs[key] = value;
}

const STYLE_SHORTHANDS = {
  bg: "background-color",
  color: "color",
  border: "border",
  radius: "border-radius",
  margin: "margin",
  m: "margin",
  gap: "gap",
  width: "width",
  height: "height",
  display: "display",
  columns: "grid-template-columns",
  pad: "padding",
  padding: "padding",
  font: "font",
  weight: "font-weight",
};

const ALLOWED_INLINE_PROPERTIES = new Set([
  "background-color", "border-radius", "font-weight", "grid-template-columns", "text-align",
]);

function safeInlineStyleValue(value) {
  const lower = value.toLowerCase();
  if (/[<>{};]/.test(value)) return false;
  if (lower.includes("expression(") || lower.includes("javascript:") || lower.includes("vbscript:") || lower.includes("@import") || lower.includes("url(")) return false;
  return value.length <= 240;
}

function splitAttrs(input) {
  const out = [];
  let buf = "";
  let quoted = false;
  for (const ch of input) {
    if (ch === "\"") quoted = !quoted;
    if (ch === " " && !quoted) {
      if (buf) out.push(buf);
      buf = "";
    } else {
      buf += ch;
    }
  }
  if (buf) out.push(buf);
  return out;
}

export function unquote(raw) {
  if (isQuoted(raw)) return raw.slice(1, -1);
  return raw;
}

export function isQuoted(raw) {
  return (raw.startsWith("\"") && raw.endsWith("\"")) || (raw.startsWith("'") && raw.endsWith("'"));
}

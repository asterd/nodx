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
      if (i > 0) attrs.attrs[token.slice(0, i)] = unquote(token.slice(i + 1));
    }
  }
  attrs.classes = [...new Set(attrs.classes)].sort();
  return attrs;
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

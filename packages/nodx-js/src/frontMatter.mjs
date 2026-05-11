import { isQuoted, unquote } from "./attrs.mjs";
import { diag } from "./diagnostics.mjs";

export function parseMeta(lines, diagnostics) {
  const [meta] = parseBlockMapping(lines, 0, 0, diagnostics, true);
  return meta;
}

function checkYamlSafety(line, lineNo, diagnostics, insideBlockScalar = false) {
  if (insideBlockScalar) return;
  const trimmed = line.trimStart();
  const unquoted = trimmed.replace(/"[^"]*"|'[^']*'/g, "");
  if (
    trimmed === "---" ||
    trimmed === "..." ||
    trimmed.startsWith("--- ") ||
    trimmed.startsWith("... ")
  ) {
    diagnostics.push(diag("NODX-E019", "fatal", "Multiple YAML documents are not supported.", lineNo, 1));
  }
  if (
    unquoted.includes("&") ||
    unquoted.includes("*") ||
    unquoted.includes("!") ||
    unquoted.includes("<<:") ||
    trimmed.startsWith("? ")
  ) {
    diagnostics.push(diag("NODX-E019", "fatal", "Forbidden YAML safe-subset construct.", lineNo, 1));
  }
  if (line.match(/^\s*\t/)) {
    diagnostics.push(diag("NODX-E019", "fatal", "Forbidden YAML indentation.", lineNo, 1));
  }
  // Forbid control characters (other than tab/newline/CR)
  for (let i = 0; i < line.length; i += 1) {
    const code = line.charCodeAt(i);
    if ((code < 0x20 && code !== 0x09 && code !== 0x0a && code !== 0x0d) || code === 0x7f) {
      diagnostics.push(diag("NODX-E019", "fatal", "Forbidden control character in YAML front matter.", lineNo, 1));
      break;
    }
  }
  const idx = trimmed.indexOf(":");
  const value = idx >= 0 ? trimmed.slice(idx + 1).trim() : trimmed.replace(/^- /, "").trim();
  const key = idx >= 0 ? trimmed.slice(0, idx).trim() : "";
  if (key === "<<" || key === "?" || key.startsWith("[") || key.startsWith("{")) {
    diagnostics.push(diag("NODX-E019", "fatal", "Forbidden YAML mapping key.", lineNo, 1));
  }
  if (value.startsWith("[") || value.startsWith("{")) {
    diagnostics.push(diag("NODX-E019", "fatal", "Flow-style YAML collections are not allowed.", lineNo, 1));
  }
  if (!isQuoted(value)) {
    const lower = value.toLowerCase();
    if ([".nan", ".inf", "+.inf", "-.inf", ".infinity", "+.infinity", "-.infinity"].includes(lower)) {
      diagnostics.push(diag("NODX-E019", "fatal", "Forbidden YAML non-finite number.", lineNo, 1));
    }
    if (/^[+-]?0[box]/i.test(value)) {
      diagnostics.push(diag("NODX-E019", "fatal", "Forbidden YAML numeric special.", lineNo, 1));
    }
    if (["yes", "no", "on", "off", "y", "n"].includes(lower)) {
      diagnostics.push(diag("NODX-E019", "fatal", "Forbidden YAML boolean alias; use true/false.", lineNo, 1));
    }
    if (/^\d{4}-\d{2}-\d{2}/.test(value)) {
      diagnostics.push(diag("NODX-E019", "fatal", "Native YAML timestamps are not supported.", lineNo, 1));
    }
  }
}

function lineIndent(line) {
  let i = 0;
  while (i < line.length && line.charCodeAt(i) === 0x20) i += 1;
  return i;
}

function parseBlockMapping(lines, start, indent, diagnostics, top) {
  const map = {};
  let i = start;
  while (i < lines.length) {
    const line = lines[i];
    if (line === undefined) break;
    if (line.trim() === "") {
      i += 1;
      continue;
    }
    const li = lineIndent(line);
    if (li < indent) break;
    if (li > indent) {
      // Skipped content from a child not captured by parent
      i += 1;
      continue;
    }
    checkYamlSafety(line, i + 2, diagnostics);
    const trimmed = line.slice(li);
    if (trimmed.startsWith("- ") || trimmed === "-") {
      break;
    }
    const idx = trimmed.indexOf(":");
    if (idx < 0) {
      i += 1;
      continue;
    }
    const key = trimmed.slice(0, idx).trim();
    const rest = trimmed.slice(idx + 1).trim();
    if (Object.prototype.hasOwnProperty.call(map, key)) {
      diagnostics.push(diag("NODX-E019", "fatal", "Duplicate front matter key.", i + 2, 1));
    }
    if (rest !== "") {
      map[key] = scalar(rest);
      i += 1;
      continue;
    }
    const next = lines[i + 1] ?? "";
    const nextIndent = lineIndent(next);
    if (next.trim() === "" || nextIndent <= indent) {
      // No nested content: treat as empty string
      map[key] = "";
      i += 1;
      continue;
    }
    const nextTrim = next.slice(nextIndent);
    if (nextTrim.startsWith("- ") || nextTrim === "-") {
      const [list, consumed] = parseBlockSequence(lines, i + 1, nextIndent, diagnostics);
      map[key] = list;
      i = consumed;
      continue;
    }
    const [child, consumed] = parseBlockMapping(lines, i + 1, nextIndent, diagnostics, false);
    map[key] = child;
    i = consumed;
  }
  if (top) return [map, i];
  return [map, i];
}

function parseBlockSequence(lines, start, indent, diagnostics) {
  const out = [];
  let i = start;
  while (i < lines.length) {
    const line = lines[i];
    if (line.trim() === "") {
      i += 1;
      continue;
    }
    const li = lineIndent(line);
    if (li !== indent) break;
    const trimmed = line.slice(li);
    if (!trimmed.startsWith("- ") && trimmed !== "-") break;
    checkYamlSafety(line, i + 2, diagnostics);
    const after = trimmed === "-" ? "" : trimmed.slice(2);
    const colon = after.indexOf(":");
    if (colon >= 0 && (colon === 0 || /[A-Za-z0-9_-]$/.test(after.slice(0, colon).trim()))) {
      const key = after.slice(0, colon).trim();
      const rest = after.slice(colon + 1).trim();
      const child = {};
      i += 1;
      if (rest !== "") {
        child[key] = scalar(rest);
      } else {
        // Possibly nested mapping/sequence after the `- key:` line
        const peek = lines[i];
        if (peek !== undefined && peek.trim() !== "") {
          const ni = lineIndent(peek);
          if (ni > indent) {
            const peekTrim = peek.slice(ni);
            if (peekTrim.startsWith("- ") || peekTrim === "-") {
              const [nested, consumed] = parseBlockSequence(lines, i, ni, diagnostics);
              child[key] = nested;
              i = consumed;
            } else {
              const [nested, consumed] = parseBlockMapping(lines, i, ni, diagnostics, false);
              child[key] = nested;
              i = consumed;
            }
          }
        }
      }
      // continue capturing more mapping fields on this list item
      while (i < lines.length) {
        const peek = lines[i];
        if (peek.trim() === "") {
          i += 1;
          continue;
        }
        const pi = lineIndent(peek);
        if (pi <= indent) break;
        const peekTrim = peek.slice(pi);
        if (peekTrim.startsWith("- ") || peekTrim === "-") break;
        const k = peekTrim.indexOf(":");
        if (k < 0) break;
        checkYamlSafety(peek, i + 2, diagnostics);
        const ck = peekTrim.slice(0, k).trim();
        const cv = peekTrim.slice(k + 1).trim();
        if (cv !== "") {
          child[ck] = scalar(cv);
          i += 1;
        } else {
          // nested under this field
          i += 1;
          const next = lines[i];
          if (next !== undefined && next.trim() !== "") {
            const ni = lineIndent(next);
            if (ni > pi) {
              const nt = next.slice(ni);
              if (nt.startsWith("- ") || nt === "-") {
                const [nl, consumed] = parseBlockSequence(lines, i, ni, diagnostics);
                child[ck] = nl;
                i = consumed;
              } else {
                const [nm, consumed] = parseBlockMapping(lines, i, ni, diagnostics, false);
                child[ck] = nm;
                i = consumed;
              }
            }
          }
        }
      }
      out.push(child);
    } else {
      out.push(scalar(after));
      i += 1;
    }
  }
  return [out, i];
}

function scalar(raw) {
  if (raw === "null") return null;
  if (raw === "true") return true;
  if (raw === "false") return false;
  if (raw.startsWith("[") && raw.endsWith("]")) {
    return raw.slice(1, -1).split(",").map((item) => item.trim()).filter(Boolean).map(scalar);
  }
  if (/^[+-]?\d+(\.\d+)?$/.test(raw)) return Number(raw);
  return unquote(raw);
}

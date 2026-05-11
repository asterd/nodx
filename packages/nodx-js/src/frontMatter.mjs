import { isQuoted, unquote } from "./attrs.mjs";
import { diag } from "./diagnostics.mjs";

export function parseMeta(lines, diagnostics) {
  const meta = {};
  let i = 0;
  while (i < lines.length) {
    const line = lines[i];
    if (!line.trim()) { i++; continue; }
    checkYamlSafety(line, i + 2, diagnostics);
    if (line.startsWith(" ")) { i++; continue; }
    const idx = line.indexOf(":");
    if (idx < 0) { i++; continue; }
    const key = line.slice(0, idx).trim();
    const rest = line.slice(idx + 1).trim();
    if (Object.prototype.hasOwnProperty.call(meta, key)) {
      diagnostics.push(diag("NODX-E019", "fatal", "Duplicate front matter key.", i + 2, 1));
    }
    if (rest) {
      meta[key] = scalar(rest);
      i++;
      continue;
    }
    const next = lines[i + 1] ?? "";
    if (next.trimStart().startsWith("- ") || next.trim() === "-") {
      const [list, consumed] = parseBlockSequence(lines, i + 1, diagnostics);
      meta[key] = list;
      i = consumed;
      continue;
    }
    const [child, consumedMap] = parseBlockMapping(lines, i + 1, diagnostics);
    meta[key] = child;
    i = consumedMap;
  }
  return meta;
}

function checkYamlSafety(line, lineNo, diagnostics) {
  const trimmed = line.trimStart();
  const unquoted = trimmed.replace(/"[^"]*"|'[^']*'/g, "");
  if (trimmed === "---" || trimmed === "..." || trimmed.startsWith("--- ") || trimmed.startsWith("... ")) {
    diagnostics.push(diag("NODX-E019", "fatal", "Multiple YAML documents are not supported.", lineNo, 1));
  }
  if (unquoted.includes("&") || unquoted.includes("*") || unquoted.includes("!") || unquoted.includes("<<:") || trimmed.startsWith("? ")) {
    diagnostics.push(diag("NODX-E019", "fatal", "Forbidden YAML safe-subset construct.", lineNo, 1));
  }
  const idx = trimmed.indexOf(":");
  const value = idx >= 0 ? trimmed.slice(idx + 1).trim() : trimmed.replace(/^- /, "").trim();
  const key = idx >= 0 ? trimmed.slice(0, idx).trim() : "";
  if (key === "<<" || key === "?" || key.startsWith("[") || key.startsWith("{")) {
    diagnostics.push(diag("NODX-E019", "fatal", "Forbidden YAML mapping key.", lineNo, 1));
  }
  if (!isQuoted(value)) {
    const lower = value.toLowerCase();
    if ([".nan", ".inf", "+.inf", "-.inf", ".infinity", "+.infinity", "-.infinity"].includes(lower)) {
      diagnostics.push(diag("NODX-E019", "fatal", "Forbidden YAML non-finite number.", lineNo, 1));
    }
    if (/^[+-]?0[bx]/i.test(value)) diagnostics.push(diag("NODX-E019", "fatal", "Forbidden YAML numeric special.", lineNo, 1));
    if (/^\d{4}-\d{2}-\d{2}/.test(value)) diagnostics.push(diag("NODX-E019", "fatal", "Native YAML timestamps are not supported.", lineNo, 1));
  }
}

function parseBlockMapping(lines, start, diagnostics) {
  const map = {};
  let i = start;
  while (i < lines.length && lines[i].startsWith("  ") && !lines[i].trimStart().startsWith("- ")) {
    checkYamlSafety(lines[i], i + 2, diagnostics);
    const trimmed = lines[i].trim();
    const j = trimmed.indexOf(":");
    if (j >= 0) map[trimmed.slice(0, j).trim()] = scalar(trimmed.slice(j + 1).trim());
    i++;
  }
  return [map, i];
}

function parseBlockSequence(lines, start, diagnostics) {
  const out = [];
  let i = start;
  while (i < lines.length) {
    const line = lines[i];
    if (!line.startsWith("  ")) break;
    const trimmed = line.trimStart();
    if (!trimmed.startsWith("- ") && trimmed !== "-") break;
    checkYamlSafety(line, i + 2, diagnostics);
    const after = trimmed === "-" ? "" : trimmed.slice(2);
    const j = after.indexOf(":");
    if (j >= 0) {
      const child = {};
      const key = after.slice(0, j).trim();
      const rest = after.slice(j + 1).trim();
      if (rest === "") {
        i++;
        while (i < lines.length && lines[i].startsWith("    ")) {
          checkYamlSafety(lines[i], i + 2, diagnostics);
          const t = lines[i].trim();
          const k = t.indexOf(":");
          if (k >= 0) child[t.slice(0, k).trim()] = scalar(t.slice(k + 1).trim());
          i++;
        }
        out.push(child);
        continue;
      }
      child[key] = scalar(rest);
      i++;
      while (i < lines.length && lines[i].startsWith("    ") && !lines[i].trimStart().startsWith("- ")) {
        checkYamlSafety(lines[i], i + 2, diagnostics);
        const t = lines[i].trim();
        const k = t.indexOf(":");
        if (k >= 0) child[t.slice(0, k).trim()] = scalar(t.slice(k + 1).trim());
        i++;
      }
      out.push(child);
    } else {
      out.push(scalar(after));
      i++;
    }
  }
  return [out, i];
}

function scalar(raw) {
  if (raw === "null") return null;
  if (raw === "true") return true;
  if (raw === "false") return false;
  if (/^-?\d+(\.\d+)?$/.test(raw)) return Number(raw);
  if (raw.startsWith("[") && raw.endsWith("]")) return raw.slice(1, -1).split(",").filter(Boolean).map((v) => scalar(v.trim()));
  return unquote(raw);
}

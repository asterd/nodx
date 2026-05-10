export function parse(input) {
  const diagnostics = [];
  if (input.startsWith("\ufeff")) diagnostics.push(diag("NODX-E018", "fatal", "Byte Order Mark is not allowed.", 1, 1));
  if (input.includes("\u0000")) diagnostics.push(diag("NODX-E002", "fatal", "U+0000 is not allowed.", 1, 1));
  const lines = input.replace(/\r\n/g, "\n").replace(/\n$/, "").split("\n");
  const meta = {};
  let start = 0;
  if (lines[0] === "---") {
    const end = lines.indexOf("---", 1);
    if (end < 0) diagnostics.push(diag("NODX-E003", "fatal", "Unclosed front matter.", 1, 1));
    else {
      Object.assign(meta, parseMeta(lines.slice(1, end), diagnostics));
      start = end + 1;
    }
  }
  meta.schema ??= "nodx/0.1";
  meta.type ??= "document";
  meta.dir ??= "auto";
  meta.language ??= "und";
  const state = { lines, pos: start, diagnostics };
  const body = parseUntil(state, null);
  meta.title ??= firstHeading(body) ?? "Untitled";
  return { body, diagnostics, meta, schema: "nodx/0.1" };
}

export function canonicalJson(doc) {
  const { body, meta, schema } = doc;
  return JSON.stringify(sortValue({ body, meta, schema }));
}

function parseUntil(state, closeFrame) {
  const out = [];
  let closed = closeFrame === null;
  while (state.pos < state.lines.length) {
    const line = state.lines[state.pos];
    if (closeFrame !== null) {
      const close = parseClose(line, closeFrame.colons);
      if (close) {
        if (close.name !== null && close.name !== closeFrame.name) {
          state.diagnostics.push(diag("NODX-E005", "error",
            "Closing label `" + close.name + "` does not match open block `" + closeFrame.name + "`.",
            state.pos + 1, 1));
        }
        state.pos++;
        closed = true;
        break;
      }
    } else if (isAnyClose(line)) {
      state.diagnostics.push(diag("NODX-E005", "error", "Unmatched block closer.", state.pos + 1, 1));
      state.pos++;
      continue;
    }
    if (line.trim() === "") {
      state.pos++;
      continue;
    }
    const opener = parseOpener(line);
    if (opener) {
      out.push(parseDelimited(state, opener));
      continue;
    }
    const heading = parseHeading(line);
    if (heading) {
      state.pos++;
      heading.attrs.attrs.level = String(heading.level);
      out.push(node("heading", heading.attrs, [], parseInlines(heading.content), null));
      continue;
    }
    if (listKind(line)) {
      out.push(parseList(state));
      continue;
    }
    if (state.pos + 1 < state.lines.length && isPipeHeader(line, state.lines[state.pos + 1])) {
      out.push(parseTable(state));
      continue;
    }
    out.push(parseParagraph(state));
  }
  if (!closed) {
    state.diagnostics.push(diag("NODX-E005", "error", "Unclosed delimited block at end of input.", Math.max(state.pos, 1), 1));
  }
  return out;
}

function parseDelimited(state, opener) {
  state.pos++;
  if (["code", "pre", "math", "style"].includes(opener.name)) {
    const start = state.pos;
    while (state.pos < state.lines.length && parseClose(state.lines[state.pos], opener.colons) === null) state.pos++;
    const text = state.lines.slice(start, state.pos).join("\n");
    if (state.pos < state.lines.length) {
      const close = parseClose(state.lines[state.pos], opener.colons);
      if (close && close.name !== null && close.name !== opener.name) {
        state.diagnostics.push(diag("NODX-E005", "error",
          "Closing label `" + close.name + "` does not match open block `" + opener.name + "`.",
          state.pos + 1, 1));
      }
      state.pos++;
    } else {
      state.diagnostics.push(diag("NODX-E005", "error", "Unclosed literal block.", start + 1, 1));
    }
    return node(opener.name, opener.attrs, [], [], text);
  }
  return node(opener.name, opener.attrs, parseUntil(state, { colons: opener.colons, name: opener.name }), [], null);
}

function parseList(state) {
  const kind = listKind(state.lines[state.pos]);
  const children = [];
  while (state.pos < state.lines.length && listKind(state.lines[state.pos]) === kind) {
    const [content, checked] = stripList(state.lines[state.pos]);
    state.pos++;
    const parts = [content];
    while (state.pos < state.lines.length && state.lines[state.pos].startsWith("  ")) {
      parts.push(state.lines[state.pos].trimStart());
      state.pos++;
    }
    const attrs = emptyAttrs();
    if (checked !== null) attrs.attrs.checked = String(checked);
    children.push(node("item", attrs, [], parseInlines(parts.join("\n")), null));
  }
  const attrs = emptyAttrs();
  attrs.attrs.kind = kind;
  return node("list", attrs, children, [], null);
}

function parseTable(state) {
  const rows = [tableRow(splitPipe(state.lines[state.pos]), true)];
  state.pos += 2;
  while (state.pos < state.lines.length && state.lines[state.pos].includes("|") && state.lines[state.pos].trim()) {
    rows.push(tableRow(splitPipe(state.lines[state.pos]), false));
    state.pos++;
  }
  return node("table", emptyAttrs(), rows, [], null);
}

function parseParagraph(state) {
  const start = state.pos++;
  while (
    state.pos < state.lines.length &&
    state.lines[state.pos].trim() !== "" &&
    !parseOpener(state.lines[state.pos]) &&
    !parseHeading(state.lines[state.pos]) &&
    !listKind(state.lines[state.pos]) &&
    !isAnyClose(state.lines[state.pos])
  ) {
    if (state.pos + 1 < state.lines.length && isPipeHeader(state.lines[state.pos], state.lines[state.pos + 1])) break;
    state.pos++;
  }
  return node("paragraph", emptyAttrs(), [], parseInlines(state.lines.slice(start, state.pos).join("\n")), null);
}

function parseMeta(lines, diagnostics) {
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
      diagnostics.push(diag("NODX-E020", "fatal", "Duplicate front matter key.", i + 2, 1));
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
  if (trimmed.startsWith("&") || trimmed.startsWith("*") || trimmed.startsWith("!!") || trimmed.startsWith("---") || trimmed.startsWith("<<:")) {
    diagnostics.push(diag("NODX-E019", "fatal", "Forbidden YAML safe-subset construct.", lineNo, 1));
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

function parseOpener(line) {
  const m = /^(:::+)([A-Za-z][A-Za-z0-9-]*)(?:\s+(\{.*\}))?$/.exec(line);
  if (!m || m[1].length < 3) return null;
  return { colons: m[1].length, name: m[2], attrs: parseAttrs(m[3] ?? "") };
}

function parseHeading(line) {
  const m = /^(#{1,6}) (.*)$/.exec(line);
  if (!m) return null;
  let content = m[2].trimEnd();
  let attrs = emptyAttrs();
  const attrStart = content.lastIndexOf(" {");
  if (attrStart >= 0 && content.endsWith("}")) {
    attrs = parseAttrs(content.slice(attrStart + 1));
    content = content.slice(0, attrStart).trimEnd();
  }
  return { level: m[1].length, content, attrs };
}

function parseAttrs(raw) {
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
    } else buf += ch;
  }
  if (buf) out.push(buf);
  return out;
}

export function parseInlines(input) {
  const out = [];
  let i = 0;
  while (i < input.length) {
    const rest = input.slice(i);
    if (rest.startsWith("`") && rest.slice(1).includes("`")) {
      const end = rest.slice(1).indexOf("`") + 1;
      out.push({ text: rest.slice(1, end), type: "code" });
      i += end + 1;
    } else if (rest.startsWith("$$") && rest.slice(2).includes("$$")) {
      const end = rest.slice(2).indexOf("$$") + 2;
      out.push({ source: rest.slice(2, end), type: "math-inline" });
      i += end + 2;
    } else if (rest.startsWith("{{") && rest.includes("}}")) {
      const end = rest.indexOf("}}");
      const [namespace, name] = rest.slice(2, end).split(".");
      out.push(name ? { name, namespace, type: "var" } : { text: rest.slice(0, end + 2), type: "text" });
      i += end + 2;
    } else if (rest.startsWith("[^") && rest.includes("]")) {
      const end = rest.indexOf("]");
      out.push({ target: rest.slice(2, end), type: "footnote-ref" });
      i += end + 1;
    } else if (rest.startsWith("[@") && rest.includes("]")) {
      const end = rest.indexOf("]");
      out.push({ target: rest.slice(2, end), type: "citation-ref" });
      i += end + 1;
    } else if (rest.startsWith("@[") && rest.includes("]")) {
      const end = rest.indexOf("]");
      out.push({ target: rest.slice(2, end), type: "ref" });
      i += end + 1;
    } else if (rest.startsWith("@{") && rest.includes("}")) {
      const end = rest.indexOf("}");
      const [kind, target] = rest.slice(2, end).split(":");
      out.push(target ? { kind, target, type: "mention" } : { text: rest.slice(0, end + 1), type: "text" });
      i += end + 1;
    } else if (rest.startsWith("**") && rest.slice(2).includes("**")) {
      const end = rest.slice(2).indexOf("**") + 2;
      out.push({ children: parseInlines(rest.slice(2, end)), type: "strong" });
      i += end + 2;
    } else if (rest.startsWith("*") && rest.slice(1).includes("*")) {
      const end = rest.slice(1).indexOf("*") + 1;
      out.push({ children: parseInlines(rest.slice(1, end)), type: "em" });
      i += end + 1;
    } else if (rest.startsWith("[") && rest.includes("]")) {
      const close = rest.indexOf("]");
      const label = rest.slice(1, close);
      const after = rest.slice(close + 1);
      if (after.startsWith("(") && after.includes(")")) {
        const end = after.indexOf(")");
        out.push({ label: parseInlines(label), target: after.slice(1, end), type: "link" });
        i += close + 1 + end + 1;
      } else if (after.startsWith("{") && after.includes("}")) {
        const end = after.indexOf("}");
        out.push({ attrs: parseAttrs(after.slice(0, end + 1)), children: parseInlines(label), type: "span" });
        i += close + 1 + end + 1;
      } else {
        pushText(out, rest[0]);
        i++;
      }
    } else {
      pushText(out, rest[0]);
      i++;
    }
  }
  return out;
}

function node(type, attrs, children, inlines, text) {
  return { attrs: attrs.attrs, children, classes: attrs.classes, id: attrs.id, inlines, text, type };
}

function emptyAttrs() {
  return { attrs: {}, classes: [], id: null };
}

function pushText(out, text) {
  const last = out[out.length - 1];
  if (last?.type === "text") last.text += text;
  else out.push({ text, type: "text" });
}

function parseClose(line, n) {
  const colons = line.length - line.replace(/^:+/, "").length;
  if (colons !== n) return null;
  const after = line.slice(n);
  if (after.trim() === "") return { name: null };
  if (after.startsWith(" ")) {
    const name = after.slice(1).trimEnd();
    if (/^[A-Za-z][A-Za-z0-9-]*$/.test(name)) return { name };
  }
  return null;
}

function isAnyClose(line) {
  const colons = line.length - line.replace(/^:+/, "").length;
  if (colons < 3) return false;
  const after = line.slice(colons);
  if (after.trim() === "") return true;
  if (after.startsWith(" ")) {
    const name = after.slice(1).trimEnd();
    return /^[A-Za-z][A-Za-z0-9-]*$/.test(name);
  }
  return false;
}

function listKind(line) {
  if (line.startsWith("- [ ] ") || line.startsWith("- [x] ")) return "task";
  if (line.startsWith("- ")) return "unordered";
  if (/^\d+\. /.test(line)) return "ordered";
  return null;
}

function stripList(line) {
  if (line.startsWith("- [ ] ")) return [line.slice(6), false];
  if (line.startsWith("- [x] ")) return [line.slice(6), true];
  if (line.startsWith("- ")) return [line.slice(2), null];
  return [line.replace(/^\d+\. /, ""), null];
}

function isPipeHeader(a, b) {
  return a.includes("|") && b.includes("-") && /^[|:\- ]+$/.test(b.trim());
}

function splitPipe(line) {
  return line.trim().replace(/^\|/, "").replace(/\|$/, "").split("|").map((x) => x.trim());
}

function tableRow(cells, header) {
  return node("row", emptyAttrs(), cells.map((cell) => {
    const attrs = emptyAttrs();
    if (header) {
      attrs.attrs.header = "true";
      attrs.attrs.scope = "col";
    }
    return node("cell", attrs, [], parseInlines(cell), null);
  }), [], null);
}

function firstHeading(nodes) {
  for (const item of nodes) {
    if (item.type === "heading") return plain(item.inlines);
    const child = firstHeading(item.children);
    if (child) return child;
  }
  return null;
}

function plain(inlines) {
  return inlines.map((x) => x.text ?? plain(x.children ?? x.label ?? [])).join("");
}

function unquote(raw) {
  if ((raw.startsWith("\"") && raw.endsWith("\"")) || (raw.startsWith("'") && raw.endsWith("'"))) return raw.slice(1, -1);
  return raw;
}

function sortValue(value) {
  if (Array.isArray(value)) return value.map(sortValue);
  if (value && typeof value === "object") {
    return Object.fromEntries(Object.keys(value).sort().map((key) => [key, sortValue(value[key])]));
  }
  return value;
}

function diag(code, severity, message, line, column) {
  return { code, column, line, message, severity, target: null };
}

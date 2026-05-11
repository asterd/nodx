import { emptyAttrs, firstHeading, node } from "./ast.mjs";
import { parseAttrs } from "./attrs.mjs";
import { diag } from "./diagnostics.mjs";
import { parseMeta } from "./frontMatter.mjs";
import { parseInlines } from "./inlineParser.mjs";

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

function parseUntil(state, closeFrame) {
  const out = [];
  let closed = closeFrame === null;
  while (state.pos < state.lines.length) {
    const line = state.lines[state.pos];
    if (closeFrame !== null) {
      const close = parseClose(line, closeFrame.colons);
      if (close) {
        if (close.name !== null && close.name !== closeFrame.name) {
          state.diagnostics.push(diag("NODX-E005", "error", "Closing label `" + close.name + "` does not match open block `" + closeFrame.name + "`.", state.pos + 1, 1));
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
  if (!closed) state.diagnostics.push(diag("NODX-E005", "error", "Unclosed delimited block at end of input.", Math.max(state.pos, 1), 1));
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
        state.diagnostics.push(diag("NODX-E005", "error", "Closing label `" + close.name + "` does not match open block `" + opener.name + "`.", state.pos + 1, 1));
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

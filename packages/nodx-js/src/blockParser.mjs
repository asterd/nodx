import { emptyAttrs, firstHeading, node } from "./ast.mjs";
import { parseAttrs } from "./attrs.mjs";
import { diag } from "./diagnostics.mjs";
import { parseMeta } from "./frontMatter.mjs";
import { parseInlines } from "./inlineParser.mjs";
import { DEFAULT_LIMITS } from "./limits.mjs";

export function parse(input, limits = DEFAULT_LIMITS) {
  const diagnostics = [];
  if (input.startsWith("\ufeff")) diagnostics.push(diag("NODX-E018", "fatal", "Byte Order Mark is not allowed.", 1, 1));
  if (input.includes("\u0000")) diagnostics.push(diag("NODX-E002", "fatal", "U+0000 is not allowed.", 1, 1));
  // Text-side resource limits — parity with `nodx_core::parse_str_with_limits`.
  // A hit emits NODX-E012 fatal so callers can short-circuit before doing the
  // line-by-line work. The packaging side already enforces its own limits in
  // `package.mjs`; the checks below cover the plain-text path that previously
  // was best-effort in JS.
  const byteLen = utf8ByteLength(input);
  if (byteLen > limits.sourceBytes) {
    diagnostics.push(diag("NODX-E012", "fatal", "Input byte size limit exceeded.", 1, 1));
  }
  const oversize = firstOversizeLine(input, limits.lineLength);
  if (oversize !== null) {
    diagnostics.push(diag("NODX-E012", "fatal", "Line length limit exceeded.", oversize, 1));
  }
  const fmBytes = frontMatterByteSize(input);
  if (fmBytes !== null && fmBytes > limits.frontMatterBytes) {
    diagnostics.push(diag("NODX-E012", "fatal", "Front matter size limit exceeded.", 1, 1));
  }
  if (diagnostics.some((d) => d.severity === "fatal")) {
    // Fatal short-circuit: return an empty Document with the baseline meta.
    // Mirrors `nodx_core::parse_str_with_limits` early-return; the field set
    // is the same the normal path produces below via `meta.schema ??= ...`.
    return {
      schema: "nodx/1.0",
      meta: { schema: "nodx/1.0", type: "document", dir: "auto", language: "und" },
      body: [],
      diagnostics,
    };
  }
  const lines = input.replace(/\r\n/g, "\n").replace(/\n$/, "").split("\n");
  const meta = {};
  let start = 0;
  let hadFrontMatter = false;
  if (lines[0] === "---") {
    const end = lines.indexOf("---", 1);
    if (end < 0) diagnostics.push(diag("NODX-E003", "fatal", "Unclosed front matter.", 1, 1));
    else {
      hadFrontMatter = true;
      Object.assign(meta, parseMeta(lines.slice(1, end), diagnostics));
      start = end + 1;
    }
  }
  meta.schema ??= "nodx/1.0";
  meta.type ??= "document";
  meta.dir ??= "auto";
  meta.language ??= "und";
  if (!hadFrontMatter) meta.profiles ??= { requires: ["core"] };
  const state = { lines, pos: start, diagnostics };
  const body = parseUntil(state, null);
  // Title inference is the renderer/agent's responsibility (see nodx-render-html
  // `derive_title`). Keeping the parser inert preserves AST equality after
  // mutating operations.
  return { body, diagnostics, meta, schema: "nodx/1.0" };
}

function parseUntil(state, closeFrame) {
  const out = [];
  let closed = closeFrame === null;
  while (state.pos < state.lines.length) {
    const line = state.lines[state.pos];
    if (closeFrame !== null) {
      const close = parseMatchingClose(line, closeFrame.colons, closeFrame.name);
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
    if (isThematicBreak(line)) {
      // Thematic break: leaf node with no children/inlines/text. The opening
      // front matter `---` is consumed above, so by here `---` is unambiguous.
      state.pos++;
      out.push(node("hr", emptyAttrs(), [], [], null));
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
    // CommonMark compatibility warnings (W030..W035). See the Rust
    // `emit_block_commonmark_warnings` for the canonical reference;
    // this implementation must agree byte-for-byte on order, line,
    // column, and message.
    emitBlockCommonmarkWarnings(state);
    out.push(parseParagraph(state));
  }
  if (!closed) state.diagnostics.push(diag("NODX-E005", "error", "Unclosed delimited block at end of input.", Math.max(state.pos, 1), 1));
  return out;
}

function parseDelimited(state, opener) {
  state.pos++;
  if (["code", "pre", "math", "style"].includes(opener.name)) {
    const start = state.pos;
    while (state.pos < state.lines.length && parseMatchingClose(state.lines[state.pos], opener.colons, opener.name) === null) state.pos++;
    const text = state.lines.slice(start, state.pos).join("\n");
    if (state.pos < state.lines.length) {
      const close = parseMatchingClose(state.lines[state.pos], opener.colons, opener.name);
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
  const aligns = splitPipeAlignments(state.lines[state.pos + 1]);
  const rows = [tableRow(splitPipe(state.lines[state.pos]), true, aligns)];
  state.pos += 2;
  while (state.pos < state.lines.length && state.lines[state.pos].includes("|") && state.lines[state.pos].trim()) {
    rows.push(tableRow(splitPipe(state.lines[state.pos]), false, aligns));
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
    !isThematicBreak(state.lines[state.pos]) &&
    !isAnyClose(state.lines[state.pos])
  ) {
    if (state.pos + 1 < state.lines.length && isPipeHeader(state.lines[state.pos], state.lines[state.pos + 1])) break;
    state.pos++;
  }
  // Inline-shape CommonMark warnings (W032/W033/W035) for the paragraph
  // slice. The Rust twin lives at `scan_inline_commonmark_warnings`.
  scanInlineCommonmarkWarnings(state.lines.slice(start, state.pos), start, state.diagnostics);
  return node("paragraph", emptyAttrs(), [], parseInlines(state.lines.slice(start, state.pos).join("\n")), null);
}

function parseOpener(line) {
  const m = /^(::+)([A-Za-z][A-Za-z0-9-]*)(?:\s+(\{.*\}))?$/.exec(line);
  if (!m || m[1].length < 2) return null;
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
  } else {
    const lightId = /^(.*) (#([A-Za-z][A-Za-z0-9-]*))$/.exec(content);
    if (lightId) {
      content = lightId[1].trimEnd();
      attrs.id = lightId[3];
    }
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

function parseMatchingClose(line, n, expectedName) {
  const close = parseClose(line, n);
  if (close) return close;
  return line === ":".repeat(n) + expectedName ? { name: expectedName } : null;
}

// A thematic break (NODX-RFC-0001 §6) is a line whose trimmed content is
// three or more repetitions of a single marker char `-`, `*`, or `_` with no
// internal whitespace. Mirrors `nodx_core::block_parser::is_thematic_break`.
function isThematicBreak(line) {
  const t = line.trim();
  if (t.length < 3) return false;
  const c = t[0];
  if (c !== "-" && c !== "*" && c !== "_") return false;
  for (let i = 0; i < t.length; i += 1) if (t[i] !== c) return false;
  return true;
}

function isAnyClose(line) {
  const colons = line.length - line.replace(/^:+/, "").length;
  if (colons < 2) return false;
  const after = line.slice(colons);
  if (after.trim() === "") return true;
  if (after.startsWith(" ")) {
    const name = after.slice(1).trimEnd();
    return /^[A-Za-z][A-Za-z0-9-]*$/.test(name);
  }
  return false;
}

// PR2 (RFC §10.3): unordered list markers are `-`, `*`, `+` followed by a
// single space. Ordered markers are decimal digits followed by `.` or `)`
// and a single space. The literal marker is *not* preserved in the AST: only
// `kind` (`unordered` / `ordered` / `task`) survives, keeping canonical JSON
// byte-stable across `- ` / `* ` / `+ ` and `1.` / `1)`.
function listKind(line) {
  if (line.startsWith("- [ ] ") || line.startsWith("- [x] ")) return "task";
  if (line.startsWith("- ") || line.startsWith("* ") || line.startsWith("+ ")) return "unordered";
  if (/^\d+[.)] /.test(line)) return "ordered";
  return null;
}

function stripList(line) {
  if (line.startsWith("- [ ] ")) return [line.slice(6), false];
  if (line.startsWith("- [x] ")) return [line.slice(6), true];
  if (line.startsWith("- ") || line.startsWith("* ") || line.startsWith("+ ")) return [line.slice(2), null];
  return [line.replace(/^\d+[.)] /, ""), null];
}

function isPipeHeader(a, b) {
  return a.includes("|") && b.includes("-") && /^[|:\- ]+$/.test(b.trim());
}

function splitPipe(line) {
  return line.trim().replace(/^\|/, "").replace(/\|$/, "").split("|").map((x) => x.trim());
}

function splitPipeAlignments(line) {
  return splitPipe(line).map((cell) => {
    const left = cell.startsWith(":");
    const right = cell.endsWith(":");
    if (left && right) return "center";
    if (left) return "left";
    if (right) return "right";
    return null;
  });
}

function tableRow(cells, header, aligns) {
  return node("row", emptyAttrs(), cells.map((cell, index) => {
    const [attrs, content] = parsePipeCellAttrs(cell);
    if (header) {
      attrs.attrs.header = "true";
      attrs.attrs.scope = "col";
    }
    if (aligns[index] && !attrs.attrs.align) attrs.attrs.align = aligns[index];
    return node("cell", attrs, [], parseInlines(content), null);
  }), [], null);
}

function parsePipeCellAttrs(cell) {
  const trimmed = cell.trimStart();
  if (!trimmed.startsWith("{")) return [emptyAttrs(), cell];
  const end = trimmed.indexOf("}");
  if (end < 0) return [emptyAttrs(), cell];
  const after = trimmed.slice(end + 1);
  if (after && !after.startsWith(" ")) return [emptyAttrs(), cell];
  const attrs = parseAttrs(trimmed.slice(0, end + 1));
  return attrsAreEmpty(attrs) ? [emptyAttrs(), cell] : [attrs, after.trimStart()];
}

function attrsAreEmpty(attrs) {
  return !attrs.id && !attrs.classes.length && !Object.keys(attrs.attrs).length && !Object.keys(attrs.styles ?? {}).length;
}

// --- CommonMark compatibility warnings (W030..W035) ---
//
// These mirror `crates/nodx-core/src/block_parser.rs` (functions
// `emit_block_commonmark_warnings`, `scan_inline_commonmark_warnings`,
// `is_footnote_definition`, `is_link_reference_definition`,
// `html_entity_length`). Any change here must land in Rust and Python
// together; `scripts/run_conformance.sh` enforces byte parity.

function emitBlockCommonmarkWarnings(state) {
  const pos = state.pos;
  if (pos >= state.lines.length) return;
  const line = state.lines[pos];

  // W030 — setext heading: next line is `=` × ≥3 and the current line
  // is non-empty text.
  if (line.trim() !== "" && pos + 1 < state.lines.length) {
    const next = state.lines[pos + 1];
    if (next.length >= 3 && /^=+$/.test(next)) {
      state.diagnostics.push(diag("NODX-W030", "warning", "Setext-style heading detected. Use '# Heading' (ATX-style) instead.", pos + 2, 1));
    }
  }

  // W031 — indented code block: 4-space indent at top level. One warning
  // per contiguous run; subsequent indented lines fold into the paragraph.
  if (line.startsWith("    ")) {
    const prevIndented = pos > 0 && state.lines[pos - 1].startsWith("    ");
    if (!prevIndented) {
      state.diagnostics.push(diag("NODX-W031", "warning", "Indented code block detected. Use '::code' fenced block instead.", pos + 1, 1));
    }
  }

  // W034 — GFM footnote definition `[^id]:`.
  if (isFootnoteDefinition(line)) {
    state.diagnostics.push(diag("NODX-W034", "warning", "Footnote definitions are not part of 1.0. Use the '::footnote' block.", pos + 1, 1));
  }
}

function scanInlineCommonmarkWarnings(lines, baseLine, diagnostics) {
  for (let offset = 0; offset < lines.length; offset += 1) {
    const line = lines[offset];
    const lineNo = baseLine + offset + 1;

    // W033 — `[ref]: url` link reference definition (paragraph-leading).
    if (offset === 0 && isLinkReferenceDefinition(line)) {
      diagnostics.push(diag("NODX-W033", "warning", "Link reference syntax not supported. Use inline links '[label](url)' instead.", lineNo, 1));
    }

    let i = 0;
    while (i < line.length) {
      const rest = line.slice(i);

      // W032 — inline image `![alt](url)`.
      if (rest.startsWith("![")) {
        const closeIdx = rest.indexOf("](");
        if (closeIdx >= 0) {
          const endIdx = rest.slice(closeIdx + 2).indexOf(")");
          if (endIdx >= 0) {
            diagnostics.push(diag("NODX-W032", "warning", "Inline image syntax not supported in 1.0. Use ':::image' block (see RFC §6) for block-level images.", lineNo, i + 1));
            i += closeIdx + 2 + endIdx + 1;
            continue;
          }
        }
      }

      // W033 — inline `[label][ref]` or `[label][]`.
      if (rest.startsWith("[") && !rest.startsWith("[^") && !rest.startsWith("[@")) {
        const close = rest.indexOf("]");
        if (close > 0 && rest.slice(close + 1).startsWith("[")) {
          const end = rest.slice(close + 1).indexOf("]");
          if (end >= 0) {
            diagnostics.push(diag("NODX-W033", "warning", "Link reference syntax not supported. Use inline links '[label](url)' instead.", lineNo, i + 1));
            i += close + 1 + end + 1;
            continue;
          }
        }
      }

      // W035 — HTML entity reference.
      if (rest.startsWith("&")) {
        const len = htmlEntityLength(rest);
        if (len !== null) {
          diagnostics.push(diag("NODX-W035", "warning", "HTML entity references are not decoded. Use the Unicode character directly.", lineNo, i + 1));
          i += len;
          continue;
        }
      }

      // Advance one code unit. UTF-16 surrogates count as two units; the
      // Rust twin advances by the char's UTF-8 length, but for the
      // pattern matchers above the difference does not affect outputs
      // because every pattern is ASCII-only.
      i += 1;
    }
  }
}

function isFootnoteDefinition(line) {
  if (!line.startsWith("[^")) return false;
  const end = line.indexOf("]", 2);
  if (end < 0) return false;
  const id = line.slice(2, end);
  if (!id || !/^[A-Za-z0-9_-]+$/.test(id)) return false;
  return line.slice(end + 1).startsWith(":");
}

function isLinkReferenceDefinition(line) {
  if (!line.startsWith("[")) return false;
  const end = line.indexOf("]", 1);
  if (end < 0) return false;
  const label = line.slice(1, end);
  if (!label || label.startsWith("^") || label.startsWith("@")) return false;
  if (!/^[A-Za-z0-9_ -]+$/.test(label)) return false;
  if (!line.slice(end + 1).startsWith(":")) return false;
  return line.slice(end + 2).trim() !== "";
}

function htmlEntityLength(rest) {
  if (rest[0] !== "&") return null;
  let i = 1;
  if (rest[i] === "#") {
    i += 1;
    const hex = rest[i] === "x" || rest[i] === "X";
    if (hex) i += 1;
    const start = i;
    const digitRe = hex ? /[0-9a-fA-F]/ : /[0-9]/;
    while (i < rest.length && digitRe.test(rest[i])) i += 1;
    if (i === start) return null;
  } else {
    const start = i;
    while (i < rest.length && /[A-Za-z0-9]/.test(rest[i])) i += 1;
    if (i === start) return null;
  }
  return rest[i] === ";" ? i + 1 : null;
}

// --- Resource limit helpers (parity with `nodx_core::parse_str_with_limits`) ---

const TEXT_ENCODER = new TextEncoder();

function utf8ByteLength(input) {
  return TEXT_ENCODER.encode(input).length;
}

function firstOversizeLine(input, max) {
  // O(n) scan: count UTF-8 bytes per line directly from code units, no
  // re-encoding of growing prefixes.
  let lineNo = 1;
  let lineBytes = 0;
  for (let i = 0; i < input.length; i += 1) {
    const code = input.charCodeAt(i);
    if (code === 0x0a) {
      if (lineBytes > max) return lineNo;
      lineBytes = 0;
      lineNo += 1;
      continue;
    }
    if (code < 0x80) {
      lineBytes += 1;
    } else if (code < 0x800) {
      lineBytes += 2;
    } else if (code >= 0xd800 && code <= 0xdbff) {
      // High surrogate: pair with the low surrogate to form a 4-byte char.
      lineBytes += 4;
      i += 1;
    } else {
      lineBytes += 3;
    }
  }
  return lineBytes > max ? lineNo : null;
}

function frontMatterByteSize(input) {
  if (!input.startsWith("---")) return null;
  const after = input.charCodeAt(3);
  if (input.length > 3 && after !== 0x0a && after !== 0x0d) return null;
  const closeIdx = input.indexOf("\n---", 3);
  if (closeIdx < 0) return null;
  return utf8ByteLength(input.slice(0, closeIdx + 4));
}

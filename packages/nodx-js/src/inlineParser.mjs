import { emptyAttrs } from "./ast.mjs";
import { mergeClassSuffix, parseAttrs } from "./attrs.mjs";

const ESCAPE_CHARS = "`*[](){}#@~^=:|_!.-+<>\\\"'";

export function parseInlines(input) {
  const out = [];
  let i = 0;
  while (i < input.length) {
    const rest = input.slice(i);
    // Code span: N backticks open, N backticks close. The opening run length
    // is preserved and the matching close must be exactly the same length,
    // CommonMark-style.
    if (rest.startsWith("`")) {
      let run = 0;
      while (run < rest.length && rest[run] === "`") run += 1;
      const closeOff = findBacktickRun(rest, run, run);
      if (closeOff !== -1) {
        const raw = rest.slice(run, closeOff);
        out.push({ text: trimCodeSpan(raw), type: "code" });
        i += closeOff + run;
        continue;
      }
      // No matching close: literal backtick. The remaining backticks in the
      // run are re-examined on the next iteration.
      pushText(out, "`");
      i += 1;
      continue;
    }
    if (rest.startsWith("$$") && rest.slice(2).includes("$$")) {
      const end = rest.slice(2).indexOf("$$") + 2;
      out.push({ source: rest.slice(2, end), type: "math-inline" });
      i += end + 2;
    } else if (rest.startsWith("{{") && rest.includes("}}")) {
      const end = rest.indexOf("}}");
      const raw = rest.slice(2, end);
      const [namespace, name] = raw.split(".");
      out.push(name ? { name, namespace, type: "var" } : raw ? { name: raw, namespace: "vars", type: "var" } : { text: rest.slice(0, end + 2), type: "text" });
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
    } else if (rest.startsWith("==") && rest.slice(2).includes("==")) {
      const end = rest.slice(2).indexOf("==") + 2;
      const suffix = parseSpanSuffix(rest.slice(end + 2));
      const item = { children: parseInlines(rest.slice(2, end)), type: "mark" };
      if (suffix.consumed > 0) item.attrs = suffix.attrs;
      out.push(item);
      i += end + 2 + suffix.consumed;
    } else if (rest.startsWith("~~") && rest.slice(2).includes("~~")) {
      const end = rest.slice(2).indexOf("~~") + 2;
      out.push({ children: parseInlines(rest.slice(2, end)), type: "strike" });
      i += end + 2;
    } else if (rest.startsWith("~") && rest.slice(1).includes("~")) {
      const end = rest.slice(1).indexOf("~") + 1;
      out.push({ children: parseInlines(rest.slice(1, end)), type: "sub" });
      i += end + 1;
    } else if (rest.startsWith("^") && rest.slice(1).includes("^")) {
      const end = rest.slice(1).indexOf("^") + 1;
      out.push({ children: parseInlines(rest.slice(1, end)), type: "sup" });
      i += end + 1;
    } else if (rest.startsWith("**") && rest.slice(2).includes("**")) {
      const end = rest.slice(2).indexOf("**") + 2;
      out.push({ children: parseInlines(rest.slice(2, end)), type: "strong" });
      i += end + 2;
    } else if (rest.startsWith("*") && rest.slice(1).includes("*")) {
      const end = rest.slice(1).indexOf("*") + 1;
      out.push({ children: parseInlines(rest.slice(1, end)), type: "em" });
      i += end + 1;
    } else if (rest.startsWith("__") || rest.startsWith("_")) {
      // Underscore emphasis (RFC §12). CommonMark "intraword underscore"
      // rule: a `_` run can open emphasis only if it is left-flanking and
      // either not right-flanking or preceded by ASCII punctuation; it can
      // close only if it is right-flanking and either not left-flanking or
      // followed by ASCII punctuation. Keeps identifiers literal.
      const run = rest.startsWith("__") ? 2 : 1;
      const openerPreceding = i === 0 ? null : input[i - 1];
      const openerFollowing = i + run < input.length ? input[i + run] : null;
      if (canOpenUnderscore(openerPreceding, openerFollowing)) {
        const closeOff = findUnderscoreClose(rest, run);
        if (closeOff !== -1) {
          const inner = rest.slice(run, run + closeOff);
          out.push({ children: parseInlines(inner), type: run === 2 ? "strong" : "em" });
          i += run + closeOff + run;
          continue;
        }
      }
      pushText(out, "_");
      i += 1;
    } else if (rest.startsWith("[[") && rest.includes("]]")) {
      const close = rest.indexOf("]]");
      const label = rest.slice(2, close);
      const suffix = parseSpanSuffix(rest.slice(close + 2));
      if (suffix.consumed > 0) {
        out.push({ attrs: suffix.attrs, children: parseInlines(label), type: "span" });
        i += close + 2 + suffix.consumed;
      } else {
        pushText(out, "[");
        i++;
      }
    } else if (rest.startsWith("[") && rest.includes("]")) {
      const close = rest.indexOf("]");
      const label = rest.slice(1, close);
      const after = rest.slice(close + 1);
      if (after.startsWith("(") && after.includes(")")) {
        const end = after.indexOf(")");
        const afterLink = after.slice(end + 1);
        let attrs = parseAttrs("");
        let consumedAttrs = 0;
        if (afterLink.startsWith("{") && afterLink.includes("}")) {
          const attrEnd = afterLink.indexOf("}");
          attrs = parseAttrs(afterLink.slice(0, attrEnd + 1));
          consumedAttrs = attrEnd + 1;
        }
        const link = { label: parseInlines(label), target: after.slice(1, end), type: "link" };
        if (consumedAttrs > 0) link.attrs = attrs;
        out.push(link);
        i += close + 1 + end + 1 + consumedAttrs;
      } else if (after.startsWith("{") && after.includes("}")) {
        const end = after.indexOf("}");
        const suffix = parseSpanSuffix(after);
        out.push({ attrs: suffix.attrs, children: parseInlines(label), type: "span" });
        i += close + 1 + suffix.consumed;
      } else {
        pushText(out, rest[0]);
        i++;
      }
    } else if (rest.startsWith("\\") && rest.length > 1) {
      const next = rest[1];
      // Hard line break: a backslash immediately before `\n` becomes
      // LineBreak and consumes both characters.
      if (next === "\n") {
        out.push({ type: "line-break" });
        i += 2;
        continue;
      }
      if (ESCAPE_CHARS.includes(next)) {
        pushText(out, next);
        i += 2;
      } else {
        pushText(out, "\\");
        i += 1;
      }
    } else {
      pushText(out, rest[0]);
      i++;
    }
  }
  return out;
}

function findBacktickRun(rest, openLen, closeLen) {
  let i = openLen;
  while (i < rest.length) {
    if (rest[i] === "`") {
      const start = i;
      while (i < rest.length && rest[i] === "`") i += 1;
      if (i - start === closeLen) return start;
    } else {
      i += 1;
    }
  }
  return -1;
}

function trimCodeSpan(raw) {
  // Normalize newlines to spaces (paragraph join may have left them), then
  // strip a single surrounding space if both sides have one and the content
  // is not all spaces. CommonMark code span normalization.
  let s = raw.replace(/\n/g, " ");
  if (s.length >= 2 && s.startsWith(" ") && s.endsWith(" ") && /[^ ]/.test(s)) {
    s = s.slice(1, -1);
  }
  return s;
}

function findUnderscoreClose(rest, run) {
  let j = run;
  while (j < rest.length) {
    if (rest[j] !== "_") {
      j += 1;
      continue;
    }
    const start = j;
    while (j < rest.length && rest[j] === "_") j += 1;
    const runLen = j - start;
    if (runLen < run) continue;
    const preceding = start > 0 ? rest[start - 1] : null;
    const following = j < rest.length ? rest[j] : null;
    if (canCloseUnderscore(preceding, following)) {
      return start - run;
    }
  }
  return -1;
}

function isLeftFlanking(preceding, following) {
  if (following === null || isAsciiWhitespace(following)) return false;
  if (preceding === null) return true;
  if (isAsciiWhitespace(preceding)) return true;
  if (isAsciiPunct(preceding)) return true;
  return false;
}

function isRightFlanking(preceding, following) {
  if (preceding === null || isAsciiWhitespace(preceding)) return false;
  if (following === null) return true;
  if (isAsciiWhitespace(following)) return true;
  if (isAsciiPunct(following)) return true;
  return false;
}

function canOpenUnderscore(preceding, following) {
  if (!isLeftFlanking(preceding, following)) return false;
  if (!isRightFlanking(preceding, following)) return true;
  return preceding !== null && isAsciiPunct(preceding);
}

function canCloseUnderscore(preceding, following) {
  if (!isRightFlanking(preceding, following)) return false;
  if (!isLeftFlanking(preceding, following)) return true;
  return following !== null && isAsciiPunct(following);
}

function isAsciiAlnum(ch) {
  return /^[A-Za-z0-9]$/.test(ch);
}

function isAsciiWhitespace(ch) {
  return ch === " " || ch === "\t" || ch === "\n" || ch === "\r";
}

function isAsciiPunct(ch) {
  // ASCII punctuation for flanking purposes: any ASCII char that is neither
  // alphanumeric nor whitespace.
  if (!ch || ch.charCodeAt(0) > 0x7f) return false;
  return !isAsciiAlnum(ch) && !isAsciiWhitespace(ch);
}

function parseSpanSuffix(input) {
  let attrs = emptyAttrs();
  let consumed = 0;
  let saw = false;
  while (consumed < input.length) {
    const rest = input.slice(consumed);
    if (rest.startsWith("{") && rest.includes("}")) {
      const end = rest.indexOf("}");
      const parsed = parseAttrs(rest.slice(0, end + 1));
      attrs.id = parsed.id ?? attrs.id;
      attrs.classes.push(...parsed.classes);
      Object.assign(attrs.attrs, parsed.attrs);
      Object.assign(attrs.styles, parsed.styles);
      consumed += end + 1;
      saw = true;
    } else if (rest.startsWith(".")) {
      const classes = rest.match(/^(\.[A-Za-z_][A-Za-z0-9_-]*)+/)?.[0] ?? "";
      if (!classes) break;
      mergeClassSuffix(attrs, classes);
      consumed += classes.length;
      saw = true;
    } else {
      break;
    }
  }
  attrs.classes = [...new Set(attrs.classes)].sort();
  if (!Object.keys(attrs.styles).length) delete attrs.styles;
  return { attrs, consumed: saw ? consumed : 0 };
}

function pushText(out, text) {
  const last = out[out.length - 1];
  if (last?.type === "text") last.text += text;
  else out.push({ text, type: "text" });
}

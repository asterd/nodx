import { DEFAULT_LIMITS } from "./limits.mjs";
import { classifyUri, ReferenceKind } from "./url.mjs";

// JavaScript port of nodx-style. Mirrors the Rust auditor for parity.

const ALLOWED_PROPERTIES = new Set([
  "background", "background-color", "background-image", "background-position",
  "background-repeat", "background-size", "border", "border-block",
  "border-block-end", "border-block-start", "border-bottom", "border-collapse",
  "border-color", "border-inline", "border-inline-end", "border-inline-start",
  "border-left", "border-radius", "border-right", "border-spacing", "border-style",
  "border-top", "border-width", "box-decoration-break", "box-shadow",
  "break-after", "break-before", "break-inside", "color", "column-count",
  "column-gap", "column-width", "columns", "direction", "display", "flex",
  "flex-basis", "flex-direction", "flex-grow", "flex-shrink", "flex-wrap",
  "font", "font-family", "font-feature-settings", "font-size", "font-style",
  "font-variant", "font-weight", "gap", "grid-column", "grid-column-gap",
  "grid-row", "grid-row-gap", "grid-template-columns", "grid-template-rows",
  "hanging-punctuation", "height", "hyphens", "justify-content", "justify-items",
  "letter-spacing", "line-height", "list-style", "list-style-position",
  "list-style-type", "margin", "margin-block", "margin-block-end",
  "margin-block-start", "margin-bottom", "margin-inline", "margin-inline-end",
  "margin-inline-start", "margin-left", "margin-right", "margin-top",
  "max-height", "max-width", "min-height", "min-width", "opacity", "orphans",
  "outline", "outline-color", "outline-offset", "outline-style", "outline-width",
  "overflow", "overflow-wrap", "padding", "padding-block", "padding-block-end",
  "padding-block-start", "padding-bottom", "padding-inline", "padding-inline-end",
  "padding-inline-start", "padding-left", "padding-right", "padding-top",
  "page-break-after", "page-break-before", "page-break-inside", "position",
  "quotes", "size", "tab-size", "table-layout", "text-align", "text-decoration",
  "text-decoration-color", "text-decoration-style", "text-decoration-thickness",
  "text-indent", "text-transform", "text-underline-offset", "vertical-align",
  "white-space", "widows", "width", "word-break", "word-spacing", "writing-mode",
]);

const ALLOWED_DISPLAY = new Set([
  "block", "inline", "inline-block", "list-item", "table", "table-row",
  "table-cell", "table-header-group", "table-row-group", "table-footer-group",
  "none", "flex", "inline-flex", "grid", "inline-grid",
]);

const ALLOWED_POSITION = new Set(["static", "relative"]);

const SAFE_TAGS = new Set([
  "a", "article", "aside", "blockquote", "body", "caption", "code", "dd", "div",
  "dl", "dt", "em", "figcaption", "figure", "h1", "h2", "h3", "h4", "h5", "h6",
  "hr", "html", "img", "li", "main", "mark", "nav", "ol", "p", "pre", "section",
  "span", "strong", "sub", "sup", "table", "tbody", "td", "th", "thead", "tr",
  "ul", "var",
]);

const STRUCTURAL_PSEUDOS = new Set([
  ":root", ":first-child", ":last-child", ":only-child", ":empty",
]);

export function auditStylesheet(input, limits = DEFAULT_LIMITS) {
  const violations = [];
  auditBreakouts(input, violations);
  for (const rule of parseRules(input)) {
    auditRule(rule, limits, violations);
  }
  return dedupe(violations);
}

export function sanitizeStylesheet(input, limits = DEFAULT_LIMITS) {
  const audit = auditStylesheet(input, limits);
  if (audit.length === 0) return escapeStyleText(input);
  if (audit.some((violation) => violation.message === "Forbidden executable or breakout content in NODS.")) {
    return "/* NODX-E027: blocked unsafe style content */";
  }
  const out = [];
  for (const rule of parseRules(input)) {
    const ruleAudit = [];
    auditRule(rule, limits, ruleAudit);
    if (ruleAudit.some((violation) => violation.severity === "error")) {
      out.push("/* NODX-E027: forbidden NODS rule omitted */");
    } else {
      out.push(escapeStyleText(rule.source));
    }
  }
  return out.length ? out.join("") : "/* NODX-E027: blocked unsafe style content */";
}

export function yamlStyleToCss(input) {
  const rules = [];
  let selector = "";
  let declarations = [];
  for (const raw of input.split(/\r?\n/)) {
    if (!raw.trim() || raw.trimStart().startsWith("#")) continue;
    if (/^\t/.test(raw)) throw new Error("YAML style indentation must use spaces.");
    const indent = raw.length - raw.trimStart().length;
    const line = raw.trimEnd();
    if (indent === 0) {
      flushRule();
      if (!line.endsWith(":")) throw new Error("YAML style top-level entries must end with `:`.");
      selector = line.slice(0, -1).trim();
      if (!selector) throw new Error("Invalid YAML style selector.");
    } else if (selector) {
      const i = line.indexOf(":");
      if (i <= 0) throw new Error("YAML style declaration must use `property: value`.");
      const property = line.slice(0, i).trim();
      const value = line.slice(i + 1).trim();
      if (!property || !value) throw new Error("YAML style declarations require a property and value.");
      declarations.push(`${property}: ${value};`);
    } else {
      throw new Error("Invalid YAML style structure.");
    }
  }
  flushRule();
  return rules.join("\n");

  function flushRule() {
    if (selector && declarations.length) rules.push(`${selector} { ${declarations.join(" ")} }`);
    selector = "";
    declarations = [];
  }
}

function parseRules(input) {
  const rules = [];
  let start = 0;
  while (start < input.length) {
    const open = input.indexOf("{", start);
    if (open < 0) break;
    let depth = 1;
    let i = open + 1;
    while (i < input.length && depth > 0) {
      const ch = input[i];
      if (ch === "{") depth += 1;
      else if (ch === "}") depth -= 1;
      i += 1;
    }
    if (depth !== 0) {
      rules.push({
        prelude: input.slice(start, open),
        declarations: input.slice(open + 1),
        source: input.slice(start),
      });
      break;
    }
    rules.push({
      prelude: input.slice(start, open),
      declarations: input.slice(open + 1, i - 1),
      source: input.slice(start, i),
    });
    start = i;
  }
  return rules;
}

function auditBreakouts(input, violations) {
  const decoded = decodeCssEscapes(input).toLowerCase();
  for (const c of [
    "</style", "<script", "<svg", "<iframe", "<object", "<embed", "vbscript:",
    "expression(", "@import",
  ]) {
    if (decoded.includes(c)) {
      violations.push(err(c, "Forbidden executable or breakout content in NODS."));
    }
  }
}

function auditRule(rule, limits, violations) {
  const prelude = rule.prelude.trim();
  if (prelude.startsWith("@")) {
    auditAtRule(prelude, violations);
    const lower = prelude.toLowerCase();
    if (lower.startsWith("@media") || lower.startsWith("@supports")) {
      for (const nested of parseRules(rule.declarations)) {
        auditRule(nested, limits, violations);
      }
      return;
    }
  } else {
    auditSelector(prelude, violations);
  }
  auditDeclarations(rule.declarations, limits, violations);
}

function auditAtRule(prelude, violations) {
  const lower = prelude.toLowerCase();
  if (lower.startsWith("@page") || lower.startsWith("@media") || lower.startsWith("@supports")) return;
  violations.push(err(prelude, "Forbidden NODS at-rule."));
}

function auditSelector(selector, violations) {
  if (selector === "") {
    violations.push(err("selector", "Missing NODS selector."));
    return;
  }
  for (const part of selector.split(",")) {
    const trimmed = part.trim();
    if (trimmed === "") {
      violations.push(err("selector", "Empty NODS selector list entry."));
      continue;
    }
    for (const token of trimmed.split(/\s+/)) {
      auditSelectorToken(token, violations);
    }
  }
}

function auditSelectorToken(token, violations) {
  if (token === "") {
    violations.push(err("selector", "Empty NODS selector token."));
    return;
  }
  if (token === ">" || token === "+" || token === "~") return;
  if (STRUCTURAL_PSEUDOS.has(token)) return;
  if (token.startsWith(":not(") && token.endsWith(")")) {
    auditSelectorToken(token.slice(5, -1), violations);
    return;
  }
  if (token.includes("::")) {
    violations.push(err(token, "Pseudo-elements are not allowed in NODS."));
    return;
  }
  if (token.includes(":")) {
    violations.push(err(token, "Interactive NODS pseudo-class is forbidden."));
    return;
  }
  if (token.includes("*") || token.includes("|")) {
    violations.push(err(token, "Forbidden NODS selector."));
    return;
  }
  const bracketStart = token.indexOf("[");
  if (bracketStart >= 0) {
    const bracketEnd = token.indexOf("]");
    if (bracketEnd < 0) {
      violations.push(err(token, "Unterminated attribute selector."));
      return;
    }
    const base = token.slice(0, bracketStart);
    const attr = token.slice(bracketStart + 1, bracketEnd);
    if (base !== "" && !isSafeSelectorBase(base)) {
      violations.push(err(base, "Unsupported NODS selector."));
      return;
    }
    if (!isSafeAttributeSelector(attr)) {
      violations.push(err(attr, "Forbidden NODS attribute selector."));
    }
    return;
  }
  if (!isSafeSelectorBase(token)) {
    violations.push(err(token, "Unsupported NODS selector."));
  }
}

function isSafeSelectorBase(token) {
  if (token === "") return false;
  const idx = firstIndexOfAny(token, [".", "#"]);
  if (idx === 0) {
    const kind = token[0];
    const rest = token.slice(1);
    const next = firstIndexOfAny(rest, [".", "#"]);
    const ident = next < 0 ? rest : rest.slice(0, next);
    if (!isIdent(ident)) return false;
    if (next < 0) return kind === "." || kind === "#";
    return isSafeSelectorBase(rest.slice(next));
  }
  if (idx > 0) {
    return isSafeTag(token.slice(0, idx)) && isSafeSelectorBase(token.slice(idx));
  }
  return isSafeTag(token);
}

function isSafeTag(token) {
  return SAFE_TAGS.has(token);
}

function firstIndexOfAny(token, chars) {
  let best = -1;
  for (const ch of chars) {
    const i = token.indexOf(ch);
    if (i >= 0 && (best < 0 || i < best)) best = i;
  }
  return best;
}

function isSafeAttributeSelector(attr) {
  let i = 0;
  let name = "";
  while (i < attr.length) {
    const c = attr[i];
    if (/[A-Za-z0-9_-]/.test(c)) {
      name += c;
      i += 1;
    } else {
      break;
    }
  }
  if (name === "" || !isIdent(name)) return false;
  const rest = attr.slice(i).trim();
  if (rest === "") {
    return ["lang", "dir", "role", "data-tag", "data-tone", "data-variant", "data-color", "data-status"].includes(name);
  }
  for (const op of ["~=", "|=", "^=", "$=", "*=", "="]) {
    if (rest.startsWith(op)) {
      const value = rest.slice(op.length).trim();
      if (
        (value.startsWith('"') && value.endsWith('"') && value.length >= 2) ||
        (value.startsWith("'") && value.endsWith("'") && value.length >= 2)
      ) {
        const inner = value.slice(1, -1);
        return inner.length > 0 && !/["'<>]/.test(inner);
      }
      return false;
    }
  }
  return false;
}

function auditDeclarations(declarations, limits, violations) {
  for (const declaration of declarations.split(";")) {
    const trimmed = declaration.trim();
    if (trimmed === "") continue;
    const idx = trimmed.indexOf(":");
    if (idx < 0) {
      violations.push(err(trimmed, "Malformed NODS declaration."));
      continue;
    }
    const property = trimmed.slice(0, idx).trim().toLowerCase();
    const value = trimmed.slice(idx + 1).trim();
    if (isForbiddenProperty(property)) {
      violations.push(err(property, "Forbidden NODS property."));
      continue;
    }
    if (!isAllowedProperty(property)) {
      violations.push(warn(property, "Unsupported NODS property."));
      continue;
    }
    auditValue(property, value, limits, violations);
  }
}

function isForbiddenProperty(property) {
  return ["behavior", "-moz-binding", "-ms-behavior", "binding"].includes(property);
}

function isAllowedProperty(property) {
  if (property.startsWith("--") && property.length > 2) return true;
  return ALLOWED_PROPERTIES.has(property);
}

function auditValue(property, value, limits, violations) {
  const decoded = decodeCssEscapes(value);
  const lower = decoded.toLowerCase();
  for (const f of ["expression(", "javascript:", "vbscript:", "@import", "behavior:"]) {
    if (lower.includes(f)) {
      violations.push(err(f, "Forbidden NODS construct in value."));
    }
  }
  for (const url of styleUrls(value)) {
    if (!classifyUri(ReferenceKind.Style, url, limits).ok) {
      violations.push(err(url, "Unsafe NODS URL."));
    }
  }
  if (property === "position" && !ALLOWED_POSITION.has(lower)) {
    violations.push(err(value, "Forbidden NODS positioning value."));
  }
  if (property === "display" && !ALLOWED_DISPLAY.has(lower)) {
    violations.push(err(value, "Forbidden NODS display value."));
  }
}

function styleUrls(input) {
  const urls = [];
  const lower = input.toLowerCase();
  let offset = 0;
  while (true) {
    const start = lower.indexOf("url(", offset);
    if (start < 0) break;
    const urlStart = start + 4;
    const end = input.indexOf(")", urlStart);
    if (end < 0) break;
    let inner = input.slice(urlStart, end).trim();
    if (
      (inner.startsWith('"') && inner.endsWith('"')) ||
      (inner.startsWith("'") && inner.endsWith("'"))
    ) {
      inner = inner.slice(1, -1);
    }
    urls.push(inner);
    offset = end + 1;
  }
  return urls;
}

function decodeCssEscapes(input) {
  let out = "";
  let i = 0;
  while (i < input.length) {
    if (input[i] === "\\" && i + 1 < input.length) {
      let j = i + 1;
      let hex = "";
      while (j < input.length && hex.length < 6 && /[0-9a-fA-F]/.test(input[j])) {
        hex += input[j];
        j += 1;
      }
      if (hex.length > 0) {
        if (j < input.length && /[ \t\n]/.test(input[j])) j += 1;
        const code = Number.parseInt(hex, 16);
        if (Number.isFinite(code)) out += String.fromCharCode(code);
        i = j;
        continue;
      }
      if (j < input.length) {
        out += input[j];
        i = j + 1;
        continue;
      }
    }
    out += input[i];
    i += 1;
  }
  return out;
}

function escapeStyleText(input) {
  return input.replace(/</g, "\\3C ").replace(/>/g, "\\3E ");
}

function dedupe(list) {
  const seen = new Set();
  const result = [];
  for (const v of list) {
    const key = v.construct + " " + v.message + " " + v.severity;
    if (!seen.has(key)) {
      seen.add(key);
      result.push(v);
    }
  }
  return result;
}

function err(construct, message) {
  return { construct: construct.trim(), message, severity: "error" };
}

function warn(construct, message) {
  return { construct: construct.trim(), message, severity: "warning" };
}

function isIdent(name) {
  if (name === "") return false;
  const first = name[0];
  if (!(/[A-Za-z_]/.test(first))) return false;
  return /^[A-Za-z_][A-Za-z0-9_-]*$/.test(name);
}

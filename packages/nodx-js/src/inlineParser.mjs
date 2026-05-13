import { emptyAttrs } from "./ast.mjs";
import { mergeClassSuffix, parseAttrs } from "./attrs.mjs";

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
    } else if (rest.startsWith("\\") && rest.length > 1 && "`*[](){}#@~^=:|".includes(rest[1])) {
      pushText(out, rest[1]);
      i += 2;
    } else {
      pushText(out, rest[0]);
      i++;
    }
  }
  return out;
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

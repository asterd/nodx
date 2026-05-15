export function node(type, attrs, children, inlines, text) {
  const out = { attrs: attrs.attrs, children, classes: attrs.classes, id: attrs.id, inlines, text, type };
  if (Object.keys(attrs.styles ?? {}).length) out.styles = attrs.styles;
  return out;
}

export function emptyAttrs() {
  return { attrs: {}, classes: [], id: null, styles: {} };
}

export function firstHeading(nodes) {
  for (const item of nodes) {
    if (item.type === "heading") return plainNodeText(item);
    const child = firstHeading(item.children);
    if (child) return child;
  }
  return null;
}

export function plainNodeText(item) {
  const parts = [plainInlines(item.inlines)];
  for (const child of item.children) {
    const text = plainNodeText(child);
    if (text) parts.push(text);
  }
  return parts.filter(Boolean).join(" ");
}

export function plainInlines(inlines) {
  let out = "";
  for (const item of inlines) {
    switch (item.type) {
      case "text":
      case "code":
        out += item.text;
        break;
      case "math-inline":
        out += item.source;
        break;
      case "strong":
      case "em":
      case "mark":
      case "strike":
      case "sub":
      case "sup":
        out += plainInlines(item.children);
        break;
      case "link":
        out += plainInlines(item.label);
        break;
      case "span":
        out += plainInlines(item.children);
        break;
      case "var":
        out += "{{" + item.namespace + "." + item.name + "}}";
        break;
      case "ref":
      case "footnote-ref":
      case "citation-ref":
        out += item.target;
        break;
      case "mention":
        out += "@" + item.kind + ":" + item.target;
        break;
      case "line-break":
        out += " ";
        break;
    }
  }
  return out;
}

export function diag(code, severity, message, line, column, target = null) {
  return { code, column, line, message, severity, target };
}

export function diagnosticsJson(diagnostics) {
  return JSON.stringify(diagnostics.map((item) => ({
    code: item.code,
    severity: item.severity,
    message: item.message,
    line: item.line,
    column: item.column,
    target: item.target,
  })));
}

export function exitCodeFor(diagnostics) {
  if (diagnostics.some((d) => d.code === "NODX-E024" && (d.severity === "fatal" || d.severity === "error"))) return 3;
  if (diagnostics.some((d) => d.severity === "fatal" || d.severity === "error")) return 2;
  return 0;
}

export function validate(doc, requestedProfile = null) {
  const diagnostics = [...doc.diagnostics];
  validateMeta(doc, requestedProfile, diagnostics);
  const components = componentNames(doc);
  const vars = new Set(doc.meta.vars && typeof doc.meta.vars === "object" && !Array.isArray(doc.meta.vars) ? Object.keys(doc.meta.vars) : []);
  const ids = new Set();
  const refs = [];
  const previousHeading = { level: 0 };
  validateNodes(doc.body, ids, refs, components, vars, previousHeading, diagnostics);
  validateTocScopes(doc.body, ids, diagnostics);
  for (const target of refs) {
    if (!ids.has(target)) {
      diagnostics.push(validationDiag("NODX-E007", "error", "Unresolved reference `#" + target + "`.", "#" + target));
    }
  }
  return diagnostics;
}

function validateMeta(doc, requestedProfile, diagnostics) {
  if (doc.meta.schema !== "nodx/0.1" && doc.meta.schema !== "nodx/1.0") {
    diagnostics.push(validationDiag("NODX-E004", "error", "Missing or invalid schema for NODX.", "schema"));
  }
  if (requestedProfile) validateRequiredProfile(requestedProfile, diagnostics);
  if (Array.isArray(doc.meta.requires)) {
    for (const profile of doc.meta.requires) if (typeof profile === "string") validateRequiredProfile(profile, diagnostics);
  }
  const profiles = doc.meta.profiles;
  if (profiles && typeof profiles === "object" && !Array.isArray(profiles)) {
    if (Array.isArray(profiles.requires)) {
      for (const profile of profiles.requires) if (typeof profile === "string") validateRequiredProfile(profile, diagnostics);
    }
    if (Array.isArray(profiles.optional)) {
      for (const profile of profiles.optional) {
        if (typeof profile === "string" && !supportsProfile(profile)) {
          diagnostics.push(validationDiag("NODX-E023", "warning", "Optional profile `" + profile + "` is unsupported.", "profile:" + profile));
        }
      }
    }
  }
}

function validateRequiredProfile(profile, diagnostics) {
  if (!supportsProfile(profile)) {
    diagnostics.push(validationDiag("NODX-E024", "error", "Required profile `" + profile + "` is unsupported.", "profile:" + profile));
  }
}

function supportsProfile(profile) {
  return ["plain", "core", "rich", "style", "package", "agent-read", "presentation", "rich-tables", "math", "media", "custom-components"].includes(profile);
}

function componentNames(doc) {
  const out = new Set();
  if (Array.isArray(doc.meta.components)) {
    for (const item of doc.meta.components) {
      if (item && typeof item === "object" && typeof item.name === "string") out.add(item.name);
    }
  }
  return out;
}

function validateNodes(nodes, ids, refs, components, vars, previousHeading, diagnostics) {
  for (const item of nodes) {
    if (item.id !== null) {
      if (!validName(item.id) || new TextEncoder().encode(item.id).length > 256) {
        diagnostics.push(validationDiag("NODX-E004", "error", "Invalid node id.", item.id));
      }
      if (ids.has(item.id)) diagnostics.push(validationDiag("NODX-E006", "error", "Duplicate node id.", item.id));
      ids.add(item.id);
    }
    validateCommonAttrs(item, diagnostics);
    if (item.type.includes("-") && !["citation-entry", "pagebreak", "speaker-notes"].includes(item.type) && !components.has(item.type) && !("fallback" in item.attrs)) {
      diagnostics.push(validationDiag("NODX-E014", "warning", "Custom component is not declared and has no explicit fallback.", item.type));
    }
    if (item.type === "heading") validateHeading(item, previousHeading, diagnostics);
    else if (item.type === "image") validateImage(item, diagnostics);
    else if (["media", "embed", "include"].includes(item.type)) validateAssetNode(item, diagnostics);
    else if (item.type === "table") validateTable(item, diagnostics);
    else if (item.type === "toc") validateToc(item, diagnostics);
    collectInlineRefs(item.inlines, refs, vars, diagnostics);
    validateNodes(item.children, ids, refs, components, vars, previousHeading, diagnostics);
  }
}

function validateCommonAttrs(item, diagnostics) {
  if ("dir" in item.attrs && !["ltr", "rtl", "auto"].includes(item.attrs.dir)) {
    diagnostics.push(validationDiag("NODX-E004", "error", "Invalid dir attribute.", item.attrs.dir));
  }
}

function validateHeading(item, previousHeading, diagnostics) {
  const level = Number.parseInt(item.attrs.level ?? "1", 10);
  if (!(level >= 1 && level <= 6)) diagnostics.push(validationDiag("NODX-E004", "error", "Heading level must be 1 through 6.", String(level)));
  if (previousHeading.level > 0 && level > previousHeading.level + 1) {
    diagnostics.push(validationDiag("NODX-E022", "warning", "Heading level jumps over an intermediate level.", String(level)));
  }
  previousHeading.level = level;
}

function validateImage(item, diagnostics) {
  if (item.attrs.decorative !== "true" && !(item.attrs.alt ?? "").trim()) {
    diagnostics.push(validationDiag("NODX-E009", "error", "Informative image requires non-empty alt text.", item.id ?? "image"));
  }
  validateAssetNode(item, diagnostics);
}

function validateAssetNode(item, diagnostics) {
  if (typeof item.attrs.src === "string" && !safeAsset(item.attrs.src)) {
    diagnostics.push(validationDiag("NODX-E008", "error", "Unresolvable asset.", item.attrs.src));
  }
}

function validateTable(item, diagnostics) {
  let width = null;
  for (const row of item.children) {
    if (row.type !== "row") continue;
    const cells = row.children.filter((child) => child.type === "cell").length;
    if (width !== null && width !== cells) diagnostics.push(validationDiag("NODX-E025", "error", "Table rows must have the same number of cells.", item.id ?? "table"));
    else if (width === null) width = cells;
  }
}

function validateToc(item, diagnostics) {
  if ("role" in item.attrs && !["primary", "local", "secondary", "breadcrumb"].includes(item.attrs.role)) {
    diagnostics.push(validationDiag("NODX-E004", "error", "Invalid toc role.", item.attrs.role));
  }
  if ("source" in item.attrs && item.attrs.source !== "document") {
    diagnostics.push(validationDiag("NODX-E004", "error", "Invalid toc source.", item.attrs.source));
  }
  if ("scope" in item.attrs && (!item.attrs.scope.startsWith("#") || item.attrs.scope.length === 1)) {
    diagnostics.push(validationDiag("NODX-E004", "error", "Invalid toc scope.", item.attrs.scope));
  }
  for (const name of ["depth", "min-level", "max-level"]) {
    if (name in item.attrs && parseLevel(item.attrs[name]) === null) {
      diagnostics.push(validationDiag("NODX-E004", "error", "Invalid toc level attribute.", item.attrs[name]));
    }
  }
  const min = parseLevel(item.attrs["min-level"]);
  const max = parseLevel(item.attrs["max-level"]);
  if (min !== null && max !== null && min > max) {
    diagnostics.push(validationDiag("NODX-E004", "error", "toc min-level must not exceed max-level.", min + ".." + max));
  }
  if (!("title" in item.attrs)) {
    diagnostics.push(validationDiag("NODX-E016", "info", "toc title omitted; deterministic accessible label will be used.", defaultNavigationLabel(item.attrs.role ?? "primary")));
  }
}

function validateTocScopes(nodes, ids, diagnostics) {
  for (const item of nodes) {
    if (item.type === "toc" && typeof item.attrs.scope === "string" && item.attrs.scope.startsWith("#")) {
      const id = item.attrs.scope.slice(1);
      if (!ids.has(id)) diagnostics.push(validationDiag("NODX-E007", "error", "Unresolved toc scope.", item.attrs.scope));
    }
    validateTocScopes(item.children, ids, diagnostics);
  }
}

function collectInlineRefs(inlines, refs, vars, diagnostics) {
  for (const item of inlines) {
    if (["strong", "em", "mark", "sub", "sup"].includes(item.type)) collectInlineRefs(item.children, refs, vars, diagnostics);
    else if (item.type === "link") {
      if (!safeLink(item.target)) diagnostics.push(validationDiag("NODX-E020", "error", "Unsafe URL or scheme.", item.target));
      collectInlineRefs(item.label, refs, vars, diagnostics);
    } else if (item.type === "span") {
      if (item.attrs?.attrs?.dir && !["ltr", "rtl", "auto"].includes(item.attrs.attrs.dir)) {
        diagnostics.push(validationDiag("NODX-E004", "error", "Invalid inline dir attribute.", item.attrs.attrs.dir));
      }
      collectInlineRefs(item.children, refs, vars, diagnostics);
    } else if (item.type === "var" && item.namespace === "vars" && !vars.has(item.name)) {
      diagnostics.push(validationDiag("NODX-E013", "warning", "Variable referenced but not declared.", item.name));
    } else if (["ref", "footnote-ref", "citation-ref"].includes(item.type)) {
      refs.push(item.target);
    }
  }
}

function validName(name) {
  return /^[A-Za-z_][A-Za-z0-9_.:-]*$/.test(name);
}

function safeAsset(src) {
  return src.length <= 2048 && !src.includes("\u0000") && !src.startsWith("../") && !src.includes("/../") && !/^[A-Za-z][A-Za-z0-9+.-]*:/.test(src);
}

function safeLink(target) {
  return target.startsWith("#") || target.startsWith("/") || /^(https?|mailto):/i.test(target) || !/^[A-Za-z][A-Za-z0-9+.-]*:/.test(target);
}

function parseLevel(value) {
  if (typeof value !== "string" || !/^[0-9]+$/.test(value)) return null;
  const level = Number.parseInt(value, 10);
  return level >= 1 && level <= 6 ? level : null;
}

function defaultNavigationLabel(role) {
  if (role === "local") return "In this section";
  if (role === "secondary") return "Secondary navigation";
  if (role === "breadcrumb") return "Breadcrumb";
  return "Table of contents";
}

function validationDiag(code, severity, message, target) {
  return { code, severity, message, line: null, column: null, target };
}

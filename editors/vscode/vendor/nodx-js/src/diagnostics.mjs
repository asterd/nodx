import { auditStylesheet } from "./nods.mjs";
import { classifyUri, ReferenceKind } from "./url.mjs";
import { canonicalJson } from "./canonical.mjs";
import { sha256Base64Url } from "./bytes.mjs";

export const SCHEMA_1_0 = "nodx/1.0";

export function diag(code, severity, message, line, column, target = null) {
  return { code, column, line, message, severity, target };
}

const SUPPORTED_PROFILES = new Set([
  "plain",
  "core",
  "rich",
  "style",
  "package",
  "agent-read",
  "remote-assets",
]);

export function diagnosticsJson(diagnostics) {
  const out = [];
  for (const d of diagnostics) {
    out.push({
      code: d.code,
      severity: d.severity,
      message: d.message,
      line: d.line,
      column: d.column,
      target: d.target,
    });
  }
  return JSON.stringify(out);
}

export function exitCodeFor(diagnostics) {
  const hasUnsupportedRequired = diagnostics.some(
    (d) =>
      d.code === "NODX-E024" && (d.severity === "fatal" || d.severity === "error"),
  );
  if (hasUnsupportedRequired) return 3;
  if (diagnostics.some((d) => d.severity === "fatal" || d.severity === "error")) {
    return 2;
  }
  return 0;
}

export function validate(doc, requestedProfile = null) {
  const diagnostics = [...doc.diagnostics];
  validateMeta(doc, requestedProfile, diagnostics);
  validateIntegrity(doc, diagnostics);
  const components = componentNames(doc);
  const vars = new Set(
    doc.meta.vars && typeof doc.meta.vars === "object" && !Array.isArray(doc.meta.vars)
      ? Object.keys(doc.meta.vars)
      : [],
  );
  const ids = new Set();
  const refs = [];
  const previousHeading = { level: 0 };
  validateNodes(doc.body, ids, refs, components, vars, previousHeading, diagnostics, documentAllowsRemoteAssets(doc));
  validateTocScopes(doc.body, ids, diagnostics);
  for (const target of refs) {
    if (!ids.has(target)) {
      diagnostics.push(
        validationDiag(
          "NODX-E007",
          "error",
          "Unresolved reference `#" + target + "`.",
          "#" + target,
        ),
      );
    }
  }
  return diagnostics;
}

export function integrityDigest(doc) {
  const unsigned = {
    ...doc,
    meta: Object.fromEntries(Object.entries(doc.meta).filter(([key]) => key !== "integrity")),
  };
  return sha256Base64Url(canonicalJson(unsigned));
}

function validateIntegrity(doc, diagnostics) {
  const integrity = doc.meta.integrity;
  if (!integrity || typeof integrity !== "object" || Array.isArray(integrity)) return;
  const alg = typeof integrity.alg === "string" ? integrity.alg : "";
  const scope = typeof integrity.scope === "string" ? integrity.scope : "canonical-ast";
  const value = typeof integrity.value === "string" ? integrity.value : "";
  if (alg !== "sha256" || scope !== "canonical-ast" || !value.startsWith("sha256-")) {
    diagnostics.push(validationDiag("NODX-E028", "error", "Invalid integrity declaration.", "integrity"));
    return;
  }
  if (value !== integrityDigest(doc)) {
    diagnostics.push(validationDiag("NODX-E028", "error", "Document integrity digest does not match.", "integrity"));
  }
}

function documentAllowsRemoteAssets(doc) {
  const features = doc.meta.features;
  if (features && typeof features === "object" && !Array.isArray(features) && features["remote-assets"] === true) {
    return true;
  }
  const profiles = doc.meta.profiles;
  if (profiles && typeof profiles === "object" && !Array.isArray(profiles)) {
    for (const key of ["requires", "optional"]) {
      if (Array.isArray(profiles[key]) && profiles[key].includes("remote-assets")) return true;
    }
  }
  return false;
}

function validateMeta(doc, requestedProfile, diagnostics) {
  const schema = doc.meta.schema;
  if (typeof schema !== "string") {
    diagnostics.push(
      validationDiag(
        "NODX-E004",
        "error",
        "Missing or invalid schema for NODX.",
        "schema",
      ),
    );
  } else if (schema !== SCHEMA_1_0) {
    diagnostics.push(
      validationDiag(
        "NODX-E004",
        "error",
        "Schema `" + schema + "` is not the frozen NODX 1.0 contract `" + SCHEMA_1_0 + "`.",
        "schema",
      ),
    );
  }
  if (typeof doc.meta.theme === "string" && !isValidTheme(doc.meta.theme)) {
    diagnostics.push(
      validationDiag("NODX-E004", "error", "Invalid theme name or path.", "theme"),
    );
  }
  if (requestedProfile) validateRequiredProfile(requestedProfile, diagnostics);
  const profiles = doc.meta.profiles;
  if (profiles && typeof profiles === "object" && !Array.isArray(profiles)) {
    if (Array.isArray(profiles.requires)) {
      for (const profile of profiles.requires) {
        if (typeof profile === "string") validateRequiredProfile(profile, diagnostics);
      }
    }
    if (Array.isArray(profiles.optional)) {
      for (const profile of profiles.optional) {
        if (typeof profile === "string" && !SUPPORTED_PROFILES.has(profile)) {
          diagnostics.push(
            validationDiag(
              "NODX-E023",
              "warning",
              "Optional profile `" + profile + "` is unsupported.",
              "profile:" + profile,
            ),
          );
        }
      }
    }
  }
}

function isValidTheme(theme) {
  return ["none", "plain", "base", "web", "print", "presentation", "docs"].includes(theme)
    || (theme.endsWith(".nodt") && !theme.startsWith("/") && !theme.includes("..") && !theme.includes("\\"));
}

function validateRequiredProfile(profile, diagnostics) {
  if (!SUPPORTED_PROFILES.has(profile)) {
    diagnostics.push(
      validationDiag(
        "NODX-E024",
        "error",
        "Required profile `" + profile + "` is unsupported.",
        "profile:" + profile,
      ),
    );
  }
}

function componentNames(doc) {
  const out = new Set();
  if (Array.isArray(doc.meta.components)) {
    for (const item of doc.meta.components) {
      if (item && typeof item === "object" && typeof item.name === "string") {
        out.add(item.name);
      }
    }
  }
  return out;
}

function validateNodes(nodes, ids, refs, components, vars, previousHeading, diagnostics, allowRemoteAssets) {
  for (const item of nodes) {
    if (item.id !== null) {
      if (!validName(item.id) || new TextEncoder().encode(item.id).length > 256) {
        diagnostics.push(
          validationDiag("NODX-E004", "error", "Invalid node id.", item.id),
        );
      }
      if (ids.has(item.id)) {
        diagnostics.push(
          validationDiag("NODX-E006", "error", "Duplicate node id.", item.id),
        );
      }
      ids.add(item.id);
    }
    validateCommonAttrs(item, diagnostics);
    if (
      item.type.includes("-") &&
      !["citation-entry", "pagebreak", "speaker-notes", "media-fallback"].includes(item.type) &&
      !components.has(item.type) &&
      !("fallback" in item.attrs)
    ) {
      diagnostics.push(
        validationDiag(
          "NODX-E014",
          "warning",
          "Custom component is not declared and has no explicit fallback.",
          item.type,
        ),
      );
    }
    if (item.type === "heading") validateHeading(item, previousHeading, diagnostics);
    else if (item.type === "image") validateImage(item, diagnostics, allowRemoteAssets);
    else if (["media", "embed"].includes(item.type)) validateAssetNode(item, diagnostics, allowRemoteAssets, ReferenceKind.MediaFallback);
    else if (item.type === "include") validateAssetNode(item, diagnostics, false, ReferenceKind.Include);
    else if (item.type === "table") validateTable(item, diagnostics);
    else if (item.type === "toc") validateToc(item, diagnostics);
    else if (item.type === "style") validateStyleBlock(item, diagnostics);
    collectInlineRefs(item.inlines, refs, vars, diagnostics);
    validateNodes(item.children, ids, refs, components, vars, previousHeading, diagnostics, allowRemoteAssets);
  }
}

function validateCommonAttrs(item, diagnostics) {
  if ("dir" in item.attrs && !["ltr", "rtl", "auto"].includes(item.attrs.dir)) {
    diagnostics.push(
      validationDiag("NODX-E004", "error", "Invalid dir attribute.", item.attrs.dir),
    );
  }
}

function validateHeading(item, previousHeading, diagnostics) {
  const level = Number.parseInt(item.attrs.level ?? "1", 10);
  if (!(level >= 1 && level <= 6)) {
    diagnostics.push(
      validationDiag(
        "NODX-E004",
        "error",
        "Heading level must be 1 through 6.",
        String(level),
      ),
    );
  }
  if (previousHeading.level > 0 && level > previousHeading.level + 1) {
    diagnostics.push(
      validationDiag(
        "NODX-E022",
        "warning",
        "Heading level jumps over an intermediate level.",
        String(level),
      ),
    );
  }
  previousHeading.level = level;
}

function validateImage(item, diagnostics, allowRemoteAssets) {
  if (item.attrs.decorative !== "true" && !(item.attrs.alt ?? "").trim()) {
    diagnostics.push(
      validationDiag(
        "NODX-E009",
        "error",
        "Informative image requires non-empty alt text.",
        item.id ?? "image",
      ),
    );
  }
  validateAssetNode(item, diagnostics, allowRemoteAssets, ReferenceKind.Asset);
}

function validateAssetNode(item, diagnostics, allowRemoteAssets, kind) {
  if (typeof item.attrs.src === "string") {
    const result = classifyUri(kind, item.attrs.src, undefined, { remoteAssets: allowRemoteAssets });
    if (!result.ok) {
      diagnostics.push(
        validationDiag("NODX-E008", "error", "Unresolvable asset.", item.attrs.src),
      );
    }
  }
}

function validateTable(item, diagnostics) {
  let width = null;
  for (const row of item.children) {
    if (row.type !== "row") continue;
    const cells = row.children
      .filter((child) => child.type === "cell")
      .reduce((sum, child) => sum + cellWidth(child), 0);
    if (width !== null && width !== cells) {
      diagnostics.push(
        validationDiag(
          "NODX-E025",
          "error",
          "Table rows must have the same number of cells.",
          item.id ?? "table",
        ),
      );
    } else if (width === null) {
      width = cells;
    }
  }
}

function cellWidth(cell) {
  const width = Number(cell.attrs?.colspan ?? 1);
  return Number.isInteger(width) && width > 0 ? width : 1;
}

function validateToc(item, diagnostics) {
  if (
    "role" in item.attrs &&
    !["primary", "local", "secondary", "breadcrumb"].includes(item.attrs.role)
  ) {
    diagnostics.push(
      validationDiag("NODX-E004", "error", "Invalid toc role.", item.attrs.role),
    );
  }
  if ("source" in item.attrs && item.attrs.source !== "document") {
    diagnostics.push(
      validationDiag("NODX-E004", "error", "Invalid toc source.", item.attrs.source),
    );
  }
  if ("mode" in item.attrs && !["auto", "manual"].includes(item.attrs.mode)) {
    diagnostics.push(
      validationDiag("NODX-E004", "error", "Invalid toc mode.", item.attrs.mode),
    );
  }
  if (
    "scope" in item.attrs &&
    (!item.attrs.scope.startsWith("#") || item.attrs.scope.length === 1)
  ) {
    diagnostics.push(
      validationDiag("NODX-E004", "error", "Invalid toc scope.", item.attrs.scope),
    );
  }
  for (const name of ["depth", "min-level", "max-level"]) {
    if (name in item.attrs && parseLevel(item.attrs[name]) === null) {
      diagnostics.push(
        validationDiag(
          "NODX-E004",
          "error",
          "Invalid toc level attribute.",
          item.attrs[name],
        ),
      );
    }
  }
  const min = parseLevel(item.attrs["min-level"]);
  const max = parseLevel(item.attrs["max-level"]);
  if (min !== null && max !== null && min > max) {
    diagnostics.push(
      validationDiag(
        "NODX-E004",
        "error",
        "toc min-level must not exceed max-level.",
        min + ".." + max,
      ),
    );
  }
  if (!("title" in item.attrs)) {
    diagnostics.push(
      validationDiag(
        "NODX-E016",
        "info",
        "toc title omitted; deterministic accessible label will be used.",
        defaultNavigationLabel(item.attrs.role ?? "primary"),
      ),
    );
  }
}

function validateStyleBlock(item, diagnostics) {
  if (typeof item.text !== "string") return;
  const source = item.attrs.format === "yaml" ? yamlStyleToCss(item.text, diagnostics, item.id) : item.text;
  if (source === null) return;
  for (const violation of auditStylesheet(source)) {
    diagnostics.push({
      code: "NODX-E027",
      severity: violation.severity,
      message: violation.message + " `" + violation.construct + "` in :::style block.",
      line: null,
      column: null,
      target: item.id ?? null,
    });
  }
}

function yamlStyleToCss(input, diagnostics, target) {
  let out = "";
  let context = null;
  for (const raw of input.split("\n")) {
    if (raw.trim() === "" || raw.trimStart().startsWith("#")) continue;
    const indent = raw.length - raw.trimStart().length;
    const line = raw.trim();
    if (indent === 0) {
      if (!line.endsWith(":")) return yamlStyleError(diagnostics, target, "YAML style top-level entries must end with `:`.");
      const name = line.slice(0, -1);
      context = ["print", "screen", "dark", "page"].includes(name)
        ? { key: name, selector: null, type: "pseudo" }
        : { selector: name, type: "selector" };
      continue;
    }
    if (context?.type === "selector" && indent >= 2) {
      const decl = splitYamlDeclaration(line, diagnostics, target);
      if (!decl) return null;
      out += (out ? "\n" : "") + context.selector + "{" + decl.property + ":" + decl.value + ";}";
    } else if (context?.type === "pseudo" && indent === 2 && line.endsWith(":")) {
      context.selector = line.slice(0, -1);
    } else if (context?.type === "pseudo" && context.selector && indent >= 4) {
      const decl = splitYamlDeclaration(line, diagnostics, target);
      if (!decl) return null;
      out += (out ? "\n" : "") + pseudoOpen(context.key) + context.selector + "{" + decl.property + ":" + decl.value + ";}}";
    } else if (context?.type === "pseudo" && context.key === "page" && indent >= 2) {
      const decl = splitYamlDeclaration(line, diagnostics, target);
      if (!decl) return null;
      out += (out ? "\n" : "") + "@page{" + decl.property + ":" + decl.value + ";}";
    } else {
      return yamlStyleError(diagnostics, target, "Invalid YAML style structure.");
    }
  }
  return out;
}

function splitYamlDeclaration(line, diagnostics, target) {
  const i = line.indexOf(":");
  if (i <= 0 || i === line.length - 1) {
    yamlStyleError(diagnostics, target, "YAML style declaration must use `property: value`.");
    return null;
  }
  return { property: line.slice(0, i).trim(), value: line.slice(i + 1).trim().replace(/^"|"$/g, "") };
}

function pseudoOpen(key) {
  if (key === "print") return "@media print{";
  if (key === "screen") return "@media screen{";
  if (key === "dark") return "@media (prefers-color-scheme: dark){";
  return "@page{";
}

function yamlStyleError(diagnostics, target, message) {
  diagnostics.push({ code: "NODX-E027", severity: "error", message, line: null, column: null, target: target ?? null });
  return null;
}

function validateTocScopes(nodes, ids, diagnostics) {
  for (const item of nodes) {
    if (
      item.type === "toc" &&
      typeof item.attrs.scope === "string" &&
      item.attrs.scope.startsWith("#")
    ) {
      const id = item.attrs.scope.slice(1);
      if (!ids.has(id)) {
        diagnostics.push(
          validationDiag(
            "NODX-E007",
            "error",
            "Unresolved toc scope.",
            item.attrs.scope,
          ),
        );
      }
    }
    validateTocScopes(item.children, ids, diagnostics);
  }
}

function collectInlineRefs(inlines, refs, vars, diagnostics) {
  for (const item of inlines) {
    if (["strong", "em", "mark", "strike", "sub", "sup"].includes(item.type)) {
      if (item.type === "mark" && item.attrs) validateInlineAttrs(item.attrs, diagnostics);
      collectInlineRefs(item.children, refs, vars, diagnostics);
    } else if (item.type === "link") {
      const result = classifyUri(ReferenceKind.Link, item.target);
      if (!result.ok) {
        diagnostics.push(
          validationDiag("NODX-E020", "error", "Unsafe URL or scheme.", item.target),
        );
      }
      collectInlineRefs(item.label, refs, vars, diagnostics);
    } else if (item.type === "span") {
      if (
        item.attrs?.attrs?.dir &&
        !["ltr", "rtl", "auto"].includes(item.attrs.attrs.dir)
      ) {
        diagnostics.push(
          validationDiag(
            "NODX-E004",
            "error",
            "Invalid inline dir attribute.",
            item.attrs.attrs.dir,
          ),
        );
      }
      collectInlineRefs(item.children, refs, vars, diagnostics);
    } else if (item.type === "var" && item.namespace === "vars" && !vars.has(item.name)) {
      diagnostics.push(
        validationDiag(
          "NODX-E013",
          "warning",
          "Variable referenced but not declared.",
          item.name,
        ),
      );
    } else if (["ref", "footnote-ref", "citation-ref"].includes(item.type)) {
      refs.push(item.target);
    }
  }
}

function validateInlineAttrs(attrs, diagnostics) {
  if (
    attrs?.attrs?.dir &&
    !["ltr", "rtl", "auto"].includes(attrs.attrs.dir)
  ) {
    diagnostics.push(
      validationDiag(
        "NODX-E004",
        "error",
        "Invalid inline dir attribute.",
        attrs.attrs.dir,
      ),
    );
  }
}

function validName(name) {
  return /^[A-Za-z_][A-Za-z0-9_.:-]*$/.test(name);
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

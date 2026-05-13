import json
import re

from .navigation import default_navigation_label
from .nods import audit_stylesheet
from .url import ReferenceKind, classify_uri

SCHEMA_1_0 = "nodx/1.0"
SUPPORTED_PROFILES = {"plain", "core", "rich", "style", "package", "agent-read"}


def diag(code, severity, message, line, column, target=None):
    return {"code": code, "column": column, "line": line, "message": message, "severity": severity, "target": target}


def diagnostics_json(diagnostics):
    out = [
        {
            "code": d["code"],
            "severity": d["severity"],
            "message": d["message"],
            "line": d["line"],
            "column": d["column"],
            "target": d["target"],
        }
        for d in diagnostics
    ]
    return json.dumps(out, ensure_ascii=False, separators=(",", ":"))


def exit_code_for(diagnostics):
    if any(d["code"] == "NODX-E024" and d["severity"] in ("fatal", "error") for d in diagnostics):
        return 3
    if any(d["severity"] in ("fatal", "error") for d in diagnostics):
        return 2
    return 0


def validate(doc, requested_profile=None):
    diagnostics = list(doc["diagnostics"])
    validate_meta(doc, requested_profile, diagnostics)
    components = component_names(doc)
    vars_ = set(doc["meta"].get("vars", {}).keys()) if isinstance(doc["meta"].get("vars"), dict) else set()
    ids = set()
    refs = []
    previous_heading = {"level": 0}
    validate_nodes(doc["body"], ids, refs, components, vars_, previous_heading, diagnostics)
    validate_toc_scopes(doc["body"], ids, diagnostics)
    for target in refs:
        if target not in ids:
            diagnostics.append(validation_diag("NODX-E007", "error", "Unresolved reference `#" + target + "`.", "#" + target))
    return diagnostics


def validate_meta(doc, requested_profile, diagnostics):
    schema = doc["meta"].get("schema")
    if not isinstance(schema, str):
        diagnostics.append(validation_diag("NODX-E004", "error", "Missing or invalid schema for NODX.", "schema"))
    elif schema != SCHEMA_1_0:
        diagnostics.append(validation_diag("NODX-E004", "error", "Schema `" + schema + "` is not the frozen NODX 1.0 contract `" + SCHEMA_1_0 + "`.", "schema"))
    theme = doc["meta"].get("theme")
    if isinstance(theme, str) and not is_valid_theme(theme):
        diagnostics.append(validation_diag("NODX-E004", "error", "Invalid theme name or path.", "theme"))
    if requested_profile:
        validate_required_profile(requested_profile, diagnostics)
    profiles = doc["meta"].get("profiles")
    if isinstance(profiles, dict):
        if isinstance(profiles.get("requires"), list):
            for profile in profiles["requires"]:
                if isinstance(profile, str):
                    validate_required_profile(profile, diagnostics)
        if isinstance(profiles.get("optional"), list):
            for profile in profiles["optional"]:
                if isinstance(profile, str) and profile not in SUPPORTED_PROFILES:
                    diagnostics.append(validation_diag("NODX-E023", "warning", "Optional profile `" + profile + "` is unsupported.", "profile:" + profile))


def is_valid_theme(theme):
    return theme in ("none", "plain", "base", "web", "print", "presentation", "docs") or (
        theme.endswith(".nodt") and not theme.startswith("/") and ".." not in theme and "\\" not in theme
    )


def validate_required_profile(profile, diagnostics):
    if profile not in SUPPORTED_PROFILES:
        diagnostics.append(validation_diag("NODX-E024", "error", "Required profile `" + profile + "` is unsupported.", "profile:" + profile))


def component_names(doc):
    out = set()
    if isinstance(doc["meta"].get("components"), list):
        for item in doc["meta"]["components"]:
            if isinstance(item, dict) and isinstance(item.get("name"), str):
                out.add(item["name"])
    return out


def validate_nodes(nodes, ids, refs, components, vars_, previous_heading, diagnostics):
    for item in nodes:
        if item["id"] is not None:
            if not valid_name(item["id"]) or len(item["id"].encode("utf-8")) > 256:
                diagnostics.append(validation_diag("NODX-E004", "error", "Invalid node id.", item["id"]))
            if item["id"] in ids:
                diagnostics.append(validation_diag("NODX-E006", "error", "Duplicate node id.", item["id"]))
            ids.add(item["id"])
        validate_common_attrs(item, diagnostics)
        if "-" in item["type"] and item["type"] not in ("citation-entry", "pagebreak", "speaker-notes", "media-fallback") and item["type"] not in components and "fallback" not in item["attrs"]:
            diagnostics.append(validation_diag("NODX-E014", "warning", "Custom component is not declared and has no explicit fallback.", item["type"]))
        if item["type"] == "heading":
            validate_heading(item, previous_heading, diagnostics)
        elif item["type"] == "image":
            validate_image(item, diagnostics)
        elif item["type"] in ("media", "embed", "include"):
            validate_asset_node(item, diagnostics)
        elif item["type"] == "table":
            validate_table(item, diagnostics)
        elif item["type"] == "toc":
            validate_toc(item, diagnostics)
        elif item["type"] == "style":
            validate_style_block(item, diagnostics)
        collect_inline_refs(item["inlines"], refs, vars_, diagnostics)
        validate_nodes(item["children"], ids, refs, components, vars_, previous_heading, diagnostics)


def validate_common_attrs(item, diagnostics):
    if "dir" in item["attrs"] and item["attrs"]["dir"] not in ("ltr", "rtl", "auto"):
        diagnostics.append(validation_diag("NODX-E004", "error", "Invalid dir attribute.", item["attrs"]["dir"]))


def validate_heading(item, previous_heading, diagnostics):
    level = int(item["attrs"].get("level", "1")) if item["attrs"].get("level", "1").isdigit() else 0
    if not (1 <= level <= 6):
        diagnostics.append(validation_diag("NODX-E004", "error", "Heading level must be 1 through 6.", str(level)))
    if previous_heading["level"] > 0 and level > previous_heading["level"] + 1:
        diagnostics.append(validation_diag("NODX-E022", "warning", "Heading level jumps over an intermediate level.", str(level)))
    previous_heading["level"] = level


def validate_image(item, diagnostics):
    if item["attrs"].get("decorative") != "true" and not item["attrs"].get("alt", "").strip():
        diagnostics.append(validation_diag("NODX-E009", "error", "Informative image requires non-empty alt text.", item["id"] or "image"))
    validate_asset_node(item, diagnostics)


def validate_asset_node(item, diagnostics):
    if isinstance(item["attrs"].get("src"), str) and not classify_uri(ReferenceKind.Asset, item["attrs"]["src"])["ok"]:
        diagnostics.append(validation_diag("NODX-E008", "error", "Unresolvable asset.", item["attrs"]["src"]))


def validate_table(item, diagnostics):
    width = None
    for row in item["children"]:
        if row["type"] != "row":
            continue
        cells = sum(cell_width(child) for child in row["children"] if child["type"] == "cell")
        if width is not None and width != cells:
            diagnostics.append(validation_diag("NODX-E025", "error", "Table rows must have the same number of cells.", item["id"] or "table"))
        elif width is None:
            width = cells


def cell_width(cell):
    value = cell["attrs"].get("colspan", "1")
    return int(value) if str(value).isdigit() and int(value) > 0 else 1


def validate_toc(item, diagnostics):
    if "role" in item["attrs"] and item["attrs"]["role"] not in ("primary", "local", "secondary", "breadcrumb"):
        diagnostics.append(validation_diag("NODX-E004", "error", "Invalid toc role.", item["attrs"]["role"]))
    if "source" in item["attrs"] and item["attrs"]["source"] != "document":
        diagnostics.append(validation_diag("NODX-E004", "error", "Invalid toc source.", item["attrs"]["source"]))
    if "mode" in item["attrs"] and item["attrs"]["mode"] not in ("auto", "manual"):
        diagnostics.append(validation_diag("NODX-E004", "error", "Invalid toc mode.", item["attrs"]["mode"]))
    if "scope" in item["attrs"] and (not item["attrs"]["scope"].startswith("#") or len(item["attrs"]["scope"]) == 1):
        diagnostics.append(validation_diag("NODX-E004", "error", "Invalid toc scope.", item["attrs"]["scope"]))
    for name in ("depth", "min-level", "max-level"):
        if name in item["attrs"] and parse_level(item["attrs"][name]) is None:
            diagnostics.append(validation_diag("NODX-E004", "error", "Invalid toc level attribute.", item["attrs"][name]))
    min_ = parse_level(item["attrs"].get("min-level"))
    max_ = parse_level(item["attrs"].get("max-level"))
    if min_ is not None and max_ is not None and min_ > max_:
        diagnostics.append(validation_diag("NODX-E004", "error", "toc min-level must not exceed max-level.", str(min_) + ".." + str(max_)))
    if "title" not in item["attrs"]:
        diagnostics.append(validation_diag("NODX-E016", "info", "toc title omitted; deterministic accessible label will be used.", default_navigation_label(item["attrs"].get("role", "primary"))))


def validate_style_block(item, diagnostics):
    if not isinstance(item["text"], str):
        return
    source = item["text"]
    if item["attrs"].get("format") == "yaml":
        source = yaml_style_to_css_for_diagnostics(item["text"], diagnostics, item["id"])
        if source is None:
            return
    for violation in audit_stylesheet(source):
        diagnostics.append(
            {
                "code": "NODX-E027",
                "severity": violation["severity"],
                "message": violation["message"] + " `" + violation["construct"] + "` in :::style block.",
                "line": None,
                "column": None,
                "target": item["id"] or None,
            }
        )


def yaml_style_to_css_for_diagnostics(input_, diagnostics, target):
    out = ""
    context = None
    for raw in input_.split("\n"):
        if raw.strip() == "" or raw.lstrip().startswith("#"):
            continue
        indent = len(raw) - len(raw.lstrip())
        line = raw.strip()
        if indent == 0:
            if not line.endswith(":"):
                return yaml_style_error(diagnostics, target, "YAML style top-level entries must end with `:`.")
            name = line[:-1]
            context = {"key": name, "selector": None, "type": "pseudo"} if name in ("print", "screen", "dark", "page") else {"selector": name, "type": "selector"}
            continue
        if context and context["type"] == "selector" and indent >= 2:
            decl = split_yaml_declaration(line, diagnostics, target)
            if not decl:
                return None
            out += ("\n" if out else "") + context["selector"] + "{" + decl["property"] + ":" + decl["value"] + ";}"
        elif context and context["type"] == "pseudo" and indent == 2 and line.endswith(":"):
            context["selector"] = line[:-1]
        elif context and context["type"] == "pseudo" and context["selector"] and indent >= 4:
            decl = split_yaml_declaration(line, diagnostics, target)
            if not decl:
                return None
            out += ("\n" if out else "") + pseudo_open(context["key"]) + context["selector"] + "{" + decl["property"] + ":" + decl["value"] + ";}}"
        elif context and context["type"] == "pseudo" and context["key"] == "page" and indent >= 2:
            decl = split_yaml_declaration(line, diagnostics, target)
            if not decl:
                return None
            out += ("\n" if out else "") + "@page{" + decl["property"] + ":" + decl["value"] + ";}"
        else:
            return yaml_style_error(diagnostics, target, "Invalid YAML style structure.")
    return out


def split_yaml_declaration(line, diagnostics, target):
    i = line.find(":")
    if i <= 0 or i == len(line) - 1:
        yaml_style_error(diagnostics, target, "YAML style declaration must use `property: value`.")
        return None
    return {"property": line[:i].strip(), "value": re.sub(r'^"|"$', "", line[i + 1 :].strip())}


def pseudo_open(key):
    if key == "print":
        return "@media print{"
    if key == "screen":
        return "@media screen{"
    if key == "dark":
        return "@media (prefers-color-scheme: dark){"
    return "@page{"


def yaml_style_error(diagnostics, target, message):
    diagnostics.append({"code": "NODX-E027", "severity": "error", "message": message, "line": None, "column": None, "target": target or None})
    return None


def validate_toc_scopes(nodes, ids, diagnostics):
    for item in nodes:
        if item["type"] == "toc" and isinstance(item["attrs"].get("scope"), str) and item["attrs"]["scope"].startswith("#"):
            id_ = item["attrs"]["scope"][1:]
            if id_ not in ids:
                diagnostics.append(validation_diag("NODX-E007", "error", "Unresolved toc scope.", item["attrs"]["scope"]))
        validate_toc_scopes(item["children"], ids, diagnostics)


def collect_inline_refs(inlines, refs, vars_, diagnostics):
    for item in inlines:
        if item["type"] in ("strong", "em", "mark", "strike", "sub", "sup"):
            if item["type"] == "mark" and item.get("attrs"):
                validate_inline_attrs(item["attrs"], diagnostics)
            collect_inline_refs(item["children"], refs, vars_, diagnostics)
        elif item["type"] == "link":
            if not classify_uri(ReferenceKind.Link, item["target"])["ok"]:
                diagnostics.append(validation_diag("NODX-E020", "error", "Unsafe URL or scheme.", item["target"]))
            collect_inline_refs(item["label"], refs, vars_, diagnostics)
        elif item["type"] == "span":
            attrs = item.get("attrs", {}).get("attrs", {})
            if attrs.get("dir") and attrs["dir"] not in ("ltr", "rtl", "auto"):
                diagnostics.append(validation_diag("NODX-E004", "error", "Invalid inline dir attribute.", attrs["dir"]))
            collect_inline_refs(item["children"], refs, vars_, diagnostics)
        elif item["type"] == "var" and item["namespace"] == "vars" and item["name"] not in vars_:
            diagnostics.append(validation_diag("NODX-E013", "warning", "Variable referenced but not declared.", item["name"]))
        elif item["type"] in ("ref", "footnote-ref", "citation-ref"):
            refs.append(item["target"])


def validate_inline_attrs(attrs, diagnostics):
    values = attrs.get("attrs", {}) if isinstance(attrs, dict) else {}
    if values.get("dir") and values["dir"] not in ("ltr", "rtl", "auto"):
        diagnostics.append(validation_diag("NODX-E004", "error", "Invalid inline dir attribute.", values["dir"]))


def valid_name(name):
    return re.match(r"^[A-Za-z_][A-Za-z0-9_.:-]*$", name) is not None


def parse_level(value):
    if not isinstance(value, str) or not value.isdigit():
        return None
    level = int(value)
    return level if 1 <= level <= 6 else None


def validation_diag(code, severity, message, target):
    return {"code": code, "severity": severity, "message": message, "line": None, "column": None, "target": target}

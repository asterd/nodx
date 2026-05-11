import re

from .limits import DEFAULT_LIMITS
from .url import ReferenceKind, classify_uri

ALLOWED_PROPERTIES = {
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
}
ALLOWED_DISPLAY = {
    "block", "inline", "inline-block", "list-item", "table", "table-row",
    "table-cell", "table-header-group", "table-row-group", "table-footer-group",
    "none", "flex", "inline-flex", "grid", "inline-grid",
}
ALLOWED_POSITION = {"static", "relative"}
SAFE_TAGS = {
    "a", "article", "aside", "blockquote", "body", "caption", "code", "dd", "div",
    "dl", "dt", "em", "figcaption", "figure", "h1", "h2", "h3", "h4", "h5", "h6",
    "hr", "html", "img", "li", "main", "mark", "nav", "ol", "p", "pre", "section",
    "span", "strong", "sub", "sup", "table", "tbody", "td", "th", "thead", "tr",
    "ul", "var",
}
STRUCTURAL_PSEUDOS = {":root", ":first-child", ":last-child", ":only-child", ":empty"}


def audit_stylesheet(input_, limits=None):
    limits = limits or DEFAULT_LIMITS
    violations = []
    audit_breakouts(input_, violations)
    for rule in parse_rules(input_):
        audit_rule(rule, limits, violations)
    return dedupe(violations)


def sanitize_stylesheet(input_, limits=None):
    limits = limits or DEFAULT_LIMITS
    audit = audit_stylesheet(input_, limits)
    if len(audit) == 0:
        return escape_style_text(input_)
    if any(v["message"] == "Forbidden executable or breakout content in NODS." for v in audit):
        return "/* NODX-E027: blocked unsafe style content */"
    out = []
    for rule in parse_rules(input_):
        rule_audit = []
        audit_rule(rule, limits, rule_audit)
        if any(v["severity"] == "error" for v in rule_audit):
            out.append("/* NODX-E027: forbidden NODS rule omitted */")
        else:
            out.append(escape_style_text(rule["source"]))
    return "".join(out) if out else "/* NODX-E027: blocked unsafe style content */"


def yaml_style_to_css(input_):
    rules = []
    selector = ""
    declarations = []

    def flush_rule():
        nonlocal selector, declarations
        if selector and declarations:
            rules.append(selector + " { " + " ".join(declarations) + " }")
        selector = ""
        declarations = []

    for raw in input_.replace("\r\n", "\n").split("\n"):
        if not raw.strip() or raw.lstrip().startswith("#"):
            continue
        if raw.startswith("\t"):
            raise ValueError("YAML style indentation must use spaces.")
        indent = len(raw) - len(raw.lstrip())
        line = raw.rstrip()
        if indent == 0:
            flush_rule()
            if not line.endswith(":"):
                raise ValueError("YAML style top-level entries must end with `:`.")
            selector = line[:-1].strip()
            if not selector:
                raise ValueError("Invalid YAML style selector.")
        elif selector:
            i = line.find(":")
            if i <= 0:
                raise ValueError("YAML style declaration must use `property: value`.")
            prop = line[:i].strip()
            value = line[i + 1 :].strip()
            if not prop or not value:
                raise ValueError("YAML style declarations require a property and value.")
            declarations.append(f"{prop}: {value};")
        else:
            raise ValueError("Invalid YAML style structure.")
    flush_rule()
    return "\n".join(rules)


def parse_rules(input_):
    rules = []
    start = 0
    while start < len(input_):
        open_ = input_.find("{", start)
        if open_ < 0:
            break
        depth = 1
        i = open_ + 1
        while i < len(input_) and depth > 0:
            if input_[i] == "{":
                depth += 1
            elif input_[i] == "}":
                depth -= 1
            i += 1
        if depth != 0:
            rules.append({"prelude": input_[start:open_], "declarations": input_[open_ + 1 :], "source": input_[start:]})
            break
        rules.append({"prelude": input_[start:open_], "declarations": input_[open_ + 1 : i - 1], "source": input_[start:i]})
        start = i
    return rules


def audit_breakouts(input_, violations):
    decoded = decode_css_escapes(input_).lower()
    for construct in ("</style", "<script", "<svg", "<iframe", "<object", "<embed", "vbscript:", "expression(", "@import"):
        if construct in decoded:
            violations.append(err(construct, "Forbidden executable or breakout content in NODS."))


def audit_rule(rule, limits, violations):
    prelude = rule["prelude"].strip()
    if prelude.startswith("@"):
        audit_at_rule(prelude, violations)
        lower = prelude.lower()
        if lower.startswith("@media") or lower.startswith("@supports"):
            for nested in parse_rules(rule["declarations"]):
                audit_rule(nested, limits, violations)
            return
    else:
        audit_selector(prelude, violations)
    audit_declarations(rule["declarations"], limits, violations)


def audit_at_rule(prelude, violations):
    lower = prelude.lower()
    if lower.startswith("@page") or lower.startswith("@media") or lower.startswith("@supports"):
        return
    violations.append(err(prelude, "Forbidden NODS at-rule."))


def audit_selector(selector, violations):
    if selector == "":
        violations.append(err("selector", "Missing NODS selector."))
        return
    for part in selector.split(","):
        trimmed = part.strip()
        if trimmed == "":
            violations.append(err("selector", "Empty NODS selector list entry."))
            continue
        for token in re.split(r"\s+", trimmed):
            audit_selector_token(token, violations)


def audit_selector_token(token, violations):
    if token == "":
        violations.append(err("selector", "Empty NODS selector token."))
        return
    if token in (">", "+", "~") or token in STRUCTURAL_PSEUDOS:
        return
    if token.startswith(":not(") and token.endswith(")"):
        audit_selector_token(token[5:-1], violations)
        return
    if "::" in token:
        violations.append(err(token, "Pseudo-elements are not allowed in NODS."))
        return
    if ":" in token:
        violations.append(err(token, "Interactive NODS pseudo-class is forbidden."))
        return
    if "*" in token or "|" in token:
        violations.append(err(token, "Forbidden NODS selector."))
        return
    bracket_start = token.find("[")
    if bracket_start >= 0:
        bracket_end = token.find("]")
        if bracket_end < 0:
            violations.append(err(token, "Unterminated attribute selector."))
            return
        base = token[:bracket_start]
        attr = token[bracket_start + 1 : bracket_end]
        if base != "" and not is_safe_selector_base(base):
            violations.append(err(base, "Unsupported NODS selector."))
            return
        if not is_safe_attribute_selector(attr):
            violations.append(err(attr, "Forbidden NODS attribute selector."))
        return
    if not is_safe_selector_base(token):
        violations.append(err(token, "Unsupported NODS selector."))


def is_safe_selector_base(token):
    if token == "":
        return False
    idx = first_index_of_any(token, [".", "#"])
    if idx == 0:
        rest = token[1:]
        next_ = first_index_of_any(rest, [".", "#"])
        ident = rest if next_ < 0 else rest[:next_]
        if not is_ident(ident):
            return False
        if next_ < 0:
            return token[0] in (".", "#")
        return is_safe_selector_base(rest[next_:])
    if idx > 0:
        return is_safe_tag(token[:idx]) and is_safe_selector_base(token[idx:])
    return is_safe_tag(token)


def first_index_of_any(token, chars):
    indexes = [token.find(ch) for ch in chars if token.find(ch) >= 0]
    return min(indexes) if indexes else -1


def is_safe_tag(token):
    return token in SAFE_TAGS


def is_safe_attribute_selector(attr):
    i = 0
    name = ""
    while i < len(attr) and re.match(r"[A-Za-z0-9_-]", attr[i]):
        name += attr[i]
        i += 1
    if name == "" or not is_ident(name):
        return False
    rest = attr[i:].strip()
    if rest == "":
        return name in {"lang", "dir", "role", "data-tag", "data-tone", "data-variant", "data-color", "data-status"}
    for op in ("~=", "|=", "^=", "$=", "*=", "="):
        if rest.startswith(op):
            value = rest[len(op) :].strip()
            if ((value.startswith('"') and value.endswith('"')) or (value.startswith("'") and value.endswith("'"))) and len(value) >= 2:
                inner = value[1:-1]
                return len(inner) > 0 and re.search(r"""["'<>]""", inner) is None
            return False
    return False


def audit_declarations(declarations, limits, violations):
    for declaration in declarations.split(";"):
        trimmed = declaration.strip()
        if trimmed == "":
            continue
        idx = trimmed.find(":")
        if idx < 0:
            violations.append(err(trimmed, "Malformed NODS declaration."))
            continue
        prop = trimmed[:idx].strip().lower()
        value = trimmed[idx + 1 :].strip()
        if prop in {"behavior", "-moz-binding", "-ms-behavior", "binding"}:
            violations.append(err(prop, "Forbidden NODS property."))
            continue
        if not (prop.startswith("--") and len(prop) > 2) and prop not in ALLOWED_PROPERTIES:
            violations.append(warn(prop, "Unsupported NODS property."))
            continue
        audit_value(prop, value, limits, violations)


def audit_value(prop, value, limits, violations):
    decoded = decode_css_escapes(value)
    lower = decoded.lower()
    for construct in ("expression(", "javascript:", "vbscript:", "@import", "behavior:"):
        if construct in lower:
            violations.append(err(construct, "Forbidden NODS construct in value."))
    for url in style_urls(value):
        if not classify_uri(ReferenceKind.Style, url, limits)["ok"]:
            violations.append(err(url, "Unsafe NODS URL."))
    if prop == "position" and lower not in ALLOWED_POSITION:
        violations.append(err(value, "Forbidden NODS positioning value."))
    if prop == "display" and lower not in ALLOWED_DISPLAY:
        violations.append(err(value, "Forbidden NODS display value."))


def style_urls(input_):
    urls = []
    lower = input_.lower()
    offset = 0
    while True:
        start = lower.find("url(", offset)
        if start < 0:
            break
        url_start = start + 4
        end = input_.find(")", url_start)
        if end < 0:
            break
        inner = input_[url_start:end].strip()
        if (inner.startswith('"') and inner.endswith('"')) or (inner.startswith("'") and inner.endswith("'")):
            inner = inner[1:-1]
        urls.append(inner)
        offset = end + 1
    return urls


def decode_css_escapes(input_):
    out = ""
    i = 0
    while i < len(input_):
        if input_[i] == "\\" and i + 1 < len(input_):
            j = i + 1
            hex_ = ""
            while j < len(input_) and len(hex_) < 6 and re.match(r"[0-9a-fA-F]", input_[j]):
                hex_ += input_[j]
                j += 1
            if hex_:
                if j < len(input_) and input_[j] in " \t\n":
                    j += 1
                out += chr(int(hex_, 16))
                i = j
                continue
            if j < len(input_):
                out += input_[j]
                i = j + 1
                continue
        out += input_[i]
        i += 1
    return out


def escape_style_text(input_):
    return input_.replace("<", "\\3C ").replace(">", "\\3E ")


def dedupe(list_):
    seen = set()
    result = []
    for v in list_:
        key = v["construct"] + "\0" + v["message"] + "\0" + v["severity"]
        if key not in seen:
            seen.add(key)
            result.append(v)
    return result


def err(construct, message):
    return {"construct": construct.strip(), "message": message, "severity": "error"}


def warn(construct, message):
    return {"construct": construct.strip(), "message": message, "severity": "warning"}


def is_ident(name):
    return re.match(r"^[A-Za-z_][A-Za-z0-9_-]*$", name) is not None


auditStylesheet = audit_stylesheet
sanitizeStylesheet = sanitize_stylesheet
yamlStyleToCss = yaml_style_to_css

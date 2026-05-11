import re

from .navigation import resolve_navigation
from .nods import sanitize_stylesheet, yaml_style_to_css
from .url import ReferenceKind, classify_uri

THEME_NAMES = ["none", "plain", "base", "web", "print", "presentation"]


def render_fragment(doc, options=None):
    options = options or {}
    navigation = resolve_navigation(doc)
    extra_styles = "".join("<style>" + sanitize_stylesheet(css) + "</style>" for css in options.get("stylesheets", []))
    return extra_styles + "".join(render_node(node, str(index), navigation, options) for index, node in enumerate(doc["body"]))


def render_html(doc, options=None):
    options = options or {}
    title = doc["meta"].get("title") if isinstance(doc["meta"].get("title"), str) else derive_title(doc["body"])
    lang = ' lang="' + escape_attr(doc["meta"]["language"]) + '"' if isinstance(doc["meta"].get("language"), str) and doc["meta"].get("language") != "und" else ""
    dir_ = ' dir="' + escape_attr(doc["meta"]["dir"]) + '"' if isinstance(doc["meta"].get("dir"), str) and doc["meta"].get("dir") != "auto" else ""
    title_html = "<title>" + escape_html(title) + "</title>" if title else ""
    return "<!doctype html><html" + lang + dir_ + '><meta charset="utf-8"><style>' + theme_stylesheet(doc["meta"].get("theme")) + "</style>" + title_html + "<body>" + render_fragment(doc, options) + "</body></html>"


def theme_stylesheet(theme="base"):
    name = theme if theme in THEME_NAMES else "base"
    if name == "none":
        return 'html[dir="rtl"]{direction:rtl}'
    if name == "plain":
        return 'html[dir="rtl"]{direction:rtl}body{font:16px/1.55 system-ui,-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;color:#1f2937}'
    tokens = standard_tokens()
    common = common_styles()
    if name == "print":
        return tokens + "body{font:11pt/1.55 var(--nodx-font-body);max-width:none;margin:0;color:var(--nodx-color-text)}@page{size:A4;margin:var(--nodx-page-margin)}h1,h2,h3{break-after:avoid}table,figure,aside{break-inside:avoid}.pagebreak{break-before:page;border:0;margin:0}" + common
    if name == "presentation":
        return tokens + "body{font:28px/1.45 var(--nodx-font-body);max-width:1100px;margin:40px auto;padding:0 28px;color:var(--nodx-color-text)}h1{font-size:2.4em}h2{font-size:1.8em}" + common
    if name == "web":
        return tokens + "body{font:16px/1.65 var(--nodx-font-body);max-width:960px;margin:32px auto;padding:0 18px;color:var(--nodx-color-text)}" + common
    return tokens + "body{font:16px/1.6 var(--nodx-font-body);max-width:920px;margin:32px auto;padding:0 16px;color:var(--nodx-color-text)}" + common


def render_semantic_text(doc):
    lines = []
    write_semantic_nodes(doc["body"], lines, [])
    return re.sub(r"\n{3,}", "\n\n", "\n".join(lines)).strip() + "\n"


def write_semantic_nodes(nodes, lines, path):
    index = 0
    for node in nodes:
        if node["type"] in ("style", "pagebreak", "toc"):
            continue
        next_path = path + [index]
        index += 1
        write_semantic_node(node, lines, next_path)


def write_semantic_node(node, lines, path):
    type_ = node["type"]
    if type_ == "heading":
        lines.append("#" * clamp(int(node["attrs"].get("level", 1)), 1, 6) + " " + plain_inlines(node["inlines"]) + semantic_id(node))
    elif type_ == "paragraph":
        push_text_line(lines, plain_inlines(node["inlines"]))
    elif type_ == "list":
        ordered = node["attrs"].get("kind") == "ordered"
        i = 1
        for item in [child for child in node["children"] if child["type"] == "item"]:
            marker = str(i) + "." if ordered else "- [x]" if item["attrs"].get("checked") == "true" else "- [ ]" if item["attrs"].get("checked") == "false" else "-"
            lines.append((marker + " " + semantic_node_text(item)).strip())
            i += 1
    elif type_ == "table":
        write_semantic_table(node, lines)
    elif type_ == "figure":
        lines.append("Figure" + semantic_id(node) + ":")
        write_semantic_nodes(node.get("children", []), lines, path)
    elif type_ == "image":
        lines.append('Image' + semantic_id(node) + ': alt="' + node["attrs"].get("alt", "") + '" src="' + node["attrs"].get("src", "") + '"')
    elif type_ == "caption":
        push_text_line(lines, "Caption: " + semantic_node_text(node))
    elif type_ in ("code", "pre", "math"):
        lang = " " + node["attrs"]["lang"] if node["attrs"].get("lang") else ""
        lines.extend(["```" + type_ + lang, node.get("text") or "", "```"])
    elif type_ == "quote":
        for line in semantic_node_text(node).splitlines():
            lines.append("> " + line)
    elif type_ == "note":
        lines.append(("Note" + semantic_attrs(node) + ": " + semantic_node_text(node)).strip())
    elif type_ == "form":
        lines.append("Form" + semantic_id(node) + ":")
        for field in [child for child in node["children"] if child["type"] == "field"]:
            lines.append("- " + (field["attrs"].get("label") or field["attrs"].get("name") or "Field") + ": " + (field["attrs"].get("value") or semantic_node_text(field)))
    elif type_ == "field":
        lines.append(field_label(node) + ": " + (node["attrs"].get("value") or semantic_node_text(node)))
    elif type_ in ("media", "embed", "include"):
        lines.append((capitalize(type_) + semantic_attrs(node) + ": " + semantic_node_text(node)).strip())
    elif type_ == "bibliography":
        lines.append("Bibliography:")
        write_semantic_nodes(node.get("children", []), lines, path)
    elif type_ == "citation-entry":
        lines.append("- " + semantic_node_text(node))
    elif "-" in type_:
        lines.append("Component " + type_ + semantic_attrs(node) + ":")
        write_semantic_nodes(node.get("children", []), lines, path)
    else:
        text = semantic_node_text(node)
        if text:
            lines.append(capitalize(type_) + semantic_attrs(node) + ": " + text)
        else:
            write_semantic_nodes(node.get("children", []), lines, path)


def write_semantic_table(node, lines):
    rows = [[cell for cell in row["children"] if cell["type"] == "cell"] for row in node["children"] if row["type"] == "row"]
    if not rows:
        lines.append("Table" + semantic_id(node) + ": empty")
        return
    lines.append("Table" + semantic_id(node) + ":")
    values = [[escape_markdown_cell(semantic_node_text(cell)) for cell in row] for row in rows]
    width = max(len(row) for row in values)
    normalized = [row + [""] * max(0, width - len(row)) for row in values]
    first_row_is_header = all(cell["attrs"].get("header") == "true" for cell in rows[0])
    header = normalized[0] if first_row_is_header else ["Column " + str(index + 1) for index in range(width)]
    lines.append("| " + " | ".join(header) + " |")
    lines.append("| " + " | ".join(["---"] * width) + " |")
    for row in normalized[1 if first_row_is_header else 0 :]:
        lines.append("| " + " | ".join(row) + " |")


def semantic_node_text(node):
    parts = []
    inline = plain_inlines(node.get("inlines", [])).strip()
    if inline:
        parts.append(inline)
    if node.get("text") is not None and node["text"].strip():
        parts.append(node["text"].strip())
    for child in node.get("children", []):
        if child["type"] in ("style", "pagebreak", "toc"):
            continue
        if child["type"] in ("row", "cell", "paragraph", "caption", "item"):
            text = semantic_node_text(child)
            if text:
                parts.append(text)
    return re.sub(r"\s+", " ", " ".join(parts)).strip()


def render_node(node, path, navigation, options):
    component_renderers = options.get("componentRenderers") or options.get("component_renderers") or {}
    custom = component_renderers.get(node["type"])
    if custom:
        return custom({"node": node, "path": path, "renderChildren": lambda: render_children(node, path, navigation, options), "renderInlines": lambda inlines: render_inlines(inlines, options), "escapeHtml": escape_html, "attrs": html_attrs(node)})
    type_ = node["type"]
    if type_ == "heading":
        level = clamp(int(node["attrs"].get("level", 1)), 1, 6)
        return f"<h{level}{html_attrs(node)}>{render_inlines(node['inlines'], options)}</h{level}>"
    if type_ == "paragraph":
        return wrap_inlines("p", node, path, navigation, options)
    if type_ == "section":
        return wrap_children("section", node, path, navigation, options)
    if type_ == "note":
        return wrap_children("aside", node, path, navigation, options)
    if type_ == "quote":
        return wrap_children("blockquote", node, path, navigation, options)
    if type_ == "list":
        return wrap_children("ol" if node["attrs"].get("kind") == "ordered" else "ul", node, path, navigation, options)
    if type_ == "item":
        return wrap_inlines("li", node, path, navigation, options)
    if type_ in ("code", "pre"):
        lang = ' data-lang="' + escape_attr(node["attrs"]["lang"]) + '"' if node["attrs"].get("lang") else ""
        return "<pre><code" + lang + ">" + escape_html(node.get("text") or "") + "</code></pre>"
    if type_ == "math":
        return '<pre class="math">' + escape_html(node.get("text") or "") + "</pre>"
    if type_ == "style":
        return "<style>" + render_style(node) + "</style>"
    if type_ == "table":
        return wrap_children("table", node, path, navigation, options)
    if type_ == "row":
        return wrap_children("tr", node, path, navigation, options)
    if type_ == "cell":
        return wrap_inlines("th" if node["attrs"].get("header") == "true" else "td", node, path, navigation, options)
    if type_ == "figure":
        return wrap_children("figure", node, path, navigation, options)
    if type_ == "caption":
        return wrap_inlines("figcaption", node, path, navigation, options)
    if type_ == "image":
        return render_image(node, options)
    if type_ == "form":
        return wrap_children("dl", node, path, navigation, options)
    if type_ == "field":
        return "<div" + html_id(node) + "><dt>" + escape_html(node["attrs"].get("label") or node["attrs"].get("name") or "Field") + "</dt><dd>" + escape_html(node["attrs"].get("value") or "") + "</dd></div>"
    if type_ == "toc":
        return render_toc(node, path, navigation)
    if type_ == "pagebreak":
        return '<hr' + html_id(node) + ' class="pagebreak">'
    if type_ in ("media", "embed", "include"):
        return render_media_fallback(node, path, navigation, options)
    if type_ == "bibliography":
        return wrap_children("ol", node, path, navigation, options)
    if type_ == "citation-entry":
        return wrap_inlines("li", node, path, navigation, options)
    if type_ == "speaker-notes":
        return '<aside' + html_attrs_without_class(node) + ' class="' + class_attr(node, "speaker-notes") + '" aria-label="Speaker notes">' + render_inlines(node["inlines"], options) + render_children(node, path, navigation, options) + "</aside>"
    if "-" in type_:
        return '<section' + html_attrs_without_class(node) + ' class="' + class_attr(node, "nodx-component nodx-component--fallback") + '" data-component="' + escape_attr(type_) + '"><p class="nodx-component__title">' + escape_html(type_) + " fallback</p>" + render_inlines(node["inlines"], options) + render_children(node, path, navigation, options) + "</section>"
    return wrap_children("div", node, path, navigation, options)


def render_children(node, path, navigation, options):
    return "".join(render_node(child, child_path(path, index), navigation, options) for index, child in enumerate(node["children"]))


def wrap_children(tag, node, path, navigation, options):
    return "<" + tag + html_attrs(node) + ">" + render_children(node, path, navigation, options) + "</" + tag + ">"


def wrap_inlines(tag, node, path, navigation, options):
    return "<" + tag + html_attrs(node) + ">" + render_inlines(node["inlines"], options) + render_children(node, path, navigation, options) + "</" + tag + ">"


def render_style(node):
    try:
        source = yaml_style_to_css(node.get("text") or "") if node["attrs"].get("format") == "yaml" else node.get("text") or ""
        return sanitize_stylesheet(source)
    except Exception:
        return "/* NODX-E027: invalid YAML style block */"


def render_image(node, options):
    alt = node["attrs"].get("alt", "")
    raw = node["attrs"].get("src", "")
    resolver = options.get("assetResolver") or options.get("asset_resolver")
    src = resolver(raw, node) if resolver else safe_image_url(raw)
    if not src:
        return '<span class="nodx-blocked-image">' + escape_html(alt or "blocked image") + "</span>"
    return '<img src="' + escape_attr(src) + '" alt="' + escape_attr(alt) + '">'


def render_toc(node, path, navigation):
    nav = next((candidate for candidate in navigation["navigations"] if candidate["tocPath"] == path), None)
    label = nav["label"] if nav else node["attrs"].get("title", "Table of contents")
    entries = nav["entries"] if nav else []
    if entries:
        items = "".join('<li data-level="' + escape_attr(str(entry["level"])) + '"><a href="#' + escape_attr(entry["id"]) + '">' + escape_html(entry["title"]) + "</a></li>" for entry in entries if is_safe_fragment_id(entry["id"]))
        return "<nav" + html_id(node) + ' aria-label="' + escape_attr(label) + '"><strong>' + escape_html(label) + "</strong><ol>" + items + "</ol></nav>"
    return "<nav" + html_id(node) + ' aria-label="' + escape_attr(label) + '"><strong>' + escape_html(label) + '</strong><p class="nodx-toc-empty">(no entries)</p></nav>'


def render_media_fallback(node, path, navigation, options):
    raw = node["attrs"].get("src", "")
    text_resolver = options.get("textAssetResolver") or options.get("text_asset_resolver")
    text = text_resolver(raw) if node["type"] == "include" and text_resolver else (node["attrs"].get("alt") or node["type"]) + (" - " + raw if raw else "")
    return "<figure" + html_attrs(node) + '><div class="media-fallback">' + escape_html(text) + "</div>" + render_children(node, path, navigation, options) + "</figure>"


def render_inlines(inlines, options=None):
    options = options or {}
    out = []
    for item in inlines:
        type_ = item["type"]
        if type_ == "text":
            out.append(escape_html(item["text"]))
        elif type_ in ("strong", "em", "mark", "sub", "sup"):
            out.append("<" + type_ + ">" + render_inlines(item["children"], options) + "</" + type_ + ">")
        elif type_ == "code":
            out.append("<code>" + escape_html(item["text"]) + "</code>")
        elif type_ == "math-inline":
            out.append('<code class="math-inline">' + escape_html(item["source"]) + "</code>")
        elif type_ == "link":
            out.append(render_link(item, options))
        elif type_ == "span":
            out.append("<span" + inline_attrs(item.get("attrs")) + ">" + render_inlines(item["children"], options) + "</span>")
        elif type_ == "var":
            out.append("<var>" + escape_html(item["namespace"]) + "." + escape_html(item["name"]) + "</var>")
        elif type_ == "ref":
            out.append('<a href="#' + escape_attr(item["target"]) + '">@' + escape_html(item["target"]) + "</a>" if is_safe_fragment_id(item["target"]) else '<span class="nodx-blocked-link">@' + escape_html(item["target"]) + "</span>")
        elif type_ in ("citation-ref", "footnote-ref"):
            out.append('<a href="#' + escape_attr(item["target"]) + '">[' + escape_html(item["target"]) + "]</a>" if is_safe_fragment_id(item["target"]) else '<span class="nodx-blocked-link">[' + escape_html(item["target"]) + "]</span>")
        elif type_ == "mention":
            out.append('<span class="mention">@' + escape_html(item["kind"]) + ":" + escape_html(item["target"]) + "</span>")
    return "".join(out)


def render_link(item, options=None):
    safe = safe_link_url(item["target"])
    label = render_inlines(item["label"], options)
    if not safe:
        return '<a class="nodx-blocked-link" data-blocked="' + escape_attr(item["target"]) + '" title="Blocked unsafe URL">' + label + "</a>"
    title = ' title="' + escape_attr(item["attrs"]["attrs"]["title"]) + '"' if item.get("attrs", {}).get("attrs", {}).get("title") else ""
    rel = item.get("attrs", {}).get("attrs", {}).get("rel", "noopener noreferrer")
    return '<a href="' + escape_attr(safe) + '"' + title + ' rel="' + escape_attr(rel) + '">' + label + "</a>"


def html_attrs(node):
    out = html_id(node)
    if node.get("classes"):
        out += ' class="' + escape_attr(" ".join(node["classes"])) + '"'
    if node["attrs"].get("lang"):
        out += ' lang="' + escape_attr(node["attrs"]["lang"]) + '"'
    if node["attrs"].get("dir") in ("ltr", "rtl", "auto"):
        out += ' dir="' + escape_attr(node["attrs"]["dir"]) + '"'
    if node["attrs"].get("title"):
        out += ' title="' + escape_attr(node["attrs"]["title"]) + '"'
    return out


def html_attrs_without_class(node):
    out = html_id(node)
    if node["attrs"].get("lang"):
        out += ' lang="' + escape_attr(node["attrs"]["lang"]) + '"'
    if node["attrs"].get("dir") in ("ltr", "rtl", "auto"):
        out += ' dir="' + escape_attr(node["attrs"]["dir"]) + '"'
    if node["attrs"].get("title"):
        out += ' title="' + escape_attr(node["attrs"]["title"]) + '"'
    return out


def inline_attrs(attrs):
    if not attrs:
        return ""
    out = ' id="' + escape_attr(attrs["id"]) + '"' if attrs.get("id") else ""
    if attrs.get("classes"):
        out += ' class="' + escape_attr(" ".join(attrs["classes"])) + '"'
    if attrs.get("attrs", {}).get("lang"):
        out += ' lang="' + escape_attr(attrs["attrs"]["lang"]) + '"'
    if attrs.get("attrs", {}).get("dir") in ("ltr", "rtl", "auto"):
        out += ' dir="' + escape_attr(attrs["attrs"]["dir"]) + '"'
    if attrs.get("attrs", {}).get("title"):
        out += ' title="' + escape_attr(attrs["attrs"]["title"]) + '"'
    return out


def html_id(node):
    return ' id="' + escape_attr(node["id"]) + '"' if node.get("id") else ""


def class_attr(node, extra):
    return escape_attr(" ".join((node.get("classes") or []) + [extra]))


def safe_link_url(raw):
    return raw if classify_uri(ReferenceKind.Link, raw)["ok"] else None


def safe_image_url(raw):
    return raw if classify_uri(ReferenceKind.Asset, raw)["ok"] else None


def child_path(prefix, index):
    return str(index) if prefix == "" else prefix + "." + str(index)


def is_safe_fragment_id(input_):
    return re.match(r"^[A-Za-z_][A-Za-z0-9_.-]*$", re.sub(r"^#", "", input_)) is not None


def derive_title(nodes):
    for node in nodes:
        if node["type"] == "heading":
            return plain_inlines(node["inlines"])
        child = derive_title(node.get("children", []))
        if child:
            return child
    return ""


def plain_inlines(inlines):
    out = ""
    for item in inlines:
        if "text" in item:
            out += item["text"]
        elif "source" in item:
            out += item["source"]
        elif "children" in item:
            out += plain_inlines(item["children"])
        elif "label" in item:
            out += plain_inlines(item["label"])
        elif "target" in item:
            out += item["target"]
    return out


def standard_tokens():
    return 'html{font-family:system-ui}html[dir="rtl"]{direction:rtl}:root{--nodx-color-text:#1f2937;--nodx-color-muted:#4b5563;--nodx-color-primary:#0f766e;--nodx-color-accent:#b91c1c;--nodx-font-body:system-ui,-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;--nodx-font-heading:var(--nodx-font-body);--nodx-font-mono:ui-monospace,SFMono-Regular,Menlo,monospace;--nodx-page-margin:22mm;--nodx-line-height:1.6;--nodx-block-gap:1rem}'


def common_styles():
    return "h1,h2,h3,h4,h5,h6{font-family:var(--nodx-font-heading);line-height:1.25;color:#0f172a;margin-top:1.4em}p{margin:0 0 1em}pre{padding:12px;background:#f5f5f5;overflow:auto;border-radius:6px}code{font-family:var(--nodx-font-mono)}aside{border-inline-start:4px solid #b57f00;padding:8px 12px;background:#fff8e6}table{border-collapse:collapse;margin:0 0 1em}td,th{border:1px solid #d1d5db;padding:6px 10px}thead th{background:#f3f4f6;text-align:start}figure{margin:1.5em 0}figcaption{font-size:.9em;color:var(--nodx-color-muted)}nav ol{padding-inline-start:1.5rem}nav strong{display:block;margin-bottom:.4em}.nodx-blocked-link,.nodx-blocked-image{color:var(--nodx-color-accent);text-decoration:line-through}.nodx-blocked-link{cursor:not-allowed}.mention{font-variant:all-small-caps}.pagebreak{border:none;border-top:1px dashed #9ca3af;margin:2em 0}.math-inline{background:#f3f4f6;padding:1px 4px;border-radius:3px}.media-fallback{border:1px dashed #d1d5db;padding:12px;border-radius:6px;color:var(--nodx-color-muted)}"


def semantic_id(node):
    return " #" + node["id"] if node.get("id") else ""


def semantic_attrs(node):
    attrs = [f'{key}="{value}"' for key, value in node.get("attrs", {}).items() if value not in ("", None) and key not in ("level", "header", "scope")]
    id_ = [f'id="{node["id"]}"'] if node.get("id") else []
    all_attrs = id_ + attrs
    return " [" + " ".join(all_attrs) + "]" if all_attrs else ""


def field_label(node):
    return node["attrs"].get("label") or node["attrs"].get("name") or "Field"


def push_text_line(lines, text):
    normalized = text.strip()
    if normalized:
        lines.append(normalized)


def escape_markdown_cell(text):
    return re.sub(r"\s+", " ", text.replace("|", "\\|")).strip()


def capitalize(text):
    return text[0].upper() + text[1:] if text else text


def clamp(value, min_, max_):
    return max(min_, min(max_, value))


def escape_html(input_):
    return str(input_ if input_ is not None else "").replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")


def escape_attr(input_):
    return escape_html(input_).replace('"', "&quot;").replace("'", "&#x27;")


renderFragment = render_fragment
renderHtml = render_html
renderSemanticText = render_semantic_text
themeStylesheet = theme_stylesheet

import re

from .block_parser import parse
from .navigation import resolve_navigation
from .nods import sanitize_stylesheet, yaml_style_to_css
from .url import ReferenceKind, classify_uri

THEME_NAMES = ["none", "plain", "base", "web", "print", "presentation", "docs"]


def render_fragment(doc, options=None):
    options = options or {}
    navigation = resolve_navigation(doc)
    stylesheets = list(doc.get("meta", {}).get("stylesheets") or []) + list(options.get("stylesheets", []))
    extra_styles = "".join("<style>" + sanitize_stylesheet(css) + "</style>" for css in stylesheets)
    component_styles = "".join("<style>" + sanitize_stylesheet(item["style"]) + "</style>" for item in component_definitions(doc) if isinstance(item.get("style"), str))
    render_options = {**options, "doc": doc, "remoteAssets": document_allows_remote_assets(doc)}
    return extra_styles + component_styles + "".join(render_node(node, str(index), navigation, render_options) for index, node in enumerate(doc["body"]))


def render_html(doc, options=None):
    options = options or {}
    title = doc["meta"].get("title") if isinstance(doc["meta"].get("title"), str) else derive_title(doc["body"])
    lang = ' lang="' + escape_attr(doc["meta"]["language"]) + '"' if isinstance(doc["meta"].get("language"), str) and doc["meta"].get("language") != "und" else ""
    dir_ = ' dir="' + escape_attr(doc["meta"]["dir"]) + '"' if isinstance(doc["meta"].get("dir"), str) and doc["meta"].get("dir") != "auto" else ""
    title_html = "<title>" + escape_html(title) + "</title>" if title else ""
    body = render_docs_body(doc, options) if is_docs_layout(doc) else "<body>" + render_fragment(doc, options) + "</body>"
    page_css = page_stylesheet(doc)
    return "<!doctype html><html" + lang + dir_ + '><meta charset="utf-8"><style>' + theme_stylesheet(doc["meta"].get("theme")) + "</style>" + ("<style>" + page_css + "</style>" if page_css else "") + title_html + body + "</html>"


def theme_stylesheet(theme="base"):
    name = theme if theme in THEME_NAMES else "base"
    if name == "none":
        return 'html[dir="rtl"]{direction:rtl}'
    if name == "plain":
        return 'html[dir="rtl"]{direction:rtl}body{font:16px/1.55 system-ui,-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;color:#1f2937;background:#ffffff}'
    tokens = standard_tokens()
    common = common_styles()
    if name == "print":
        return tokens + "body{font:11pt/1.55 var(--nodx-font-body);max-width:none;margin:0;color:var(--nodx-color-text);background:var(--nodx-color-bg)}@page{size:A4;margin:var(--nodx-page-margin)}h1,h2,h3{break-after:avoid}table,figure,aside{break-inside:avoid}.pagebreak{break-before:page;border:0;margin:0}" + common
    if name == "presentation":
        return tokens + "body{font:28px/1.45 var(--nodx-font-body);max-width:1100px;margin:40px auto;padding:0 28px;color:var(--nodx-color-text);background:var(--nodx-color-bg)}h1{font-size:2.4em}h2{font-size:1.8em}" + common
    if name == "web":
        return tokens + "body{font:16px/1.65 var(--nodx-font-body);max-width:960px;margin:32px auto;padding:0 18px;color:var(--nodx-color-text);background:var(--nodx-color-bg)}" + common
    if name == "docs":
        return tokens + docs_styles() + common
    return tokens + "body{font:16px/1.6 var(--nodx-font-body);max-width:920px;margin:32px auto;padding:0 16px;color:var(--nodx-color-text);background:var(--nodx-color-bg)}" + common


def render_docs_body(doc, options):
    navigation = resolve_navigation(doc)
    entries = navigation["navigations"][0]["entries"] if navigation["navigations"] else collect_heading_entries(doc["body"])
    sidebar = docs_nav([entry for entry in entries if entry["level"] == 1])
    outline = docs_nav([entry for entry in entries if entry["level"] > 1])
    return '<body class="nodx-docs-layout"><aside class="nodx-docs-sidebar"><a class="nodx-docs-brand" href="#">' + escape_html(docs_title(doc)) + "</a>" + sidebar + '</aside><main class="nodx-docs-main">' + render_fragment(doc, options) + '</main><aside class="nodx-docs-outline">' + outline + "</aside></body>"


def docs_nav(entries):
    items = "".join('<li data-level="' + escape_attr(str(entry["level"])) + '"><a href="#' + escape_attr(entry["id"]) + '">' + escape_html(entry["title"]) + "</a></li>" for entry in entries if entry.get("id"))
    return "<nav><ol>" + items + "</ol></nav>" if items else ""


def collect_heading_entries(nodes, prefix="", out=None):
    out = out if out is not None else []
    for index, node in enumerate(nodes):
        path = child_path(prefix, index)
        if node["type"] == "heading":
            out.append({"id": node.get("id") or "", "level": int(node["attrs"].get("level", 1)), "path": path, "title": plain_inlines(node["inlines"])})
        collect_heading_entries(node.get("children", []), path, out)
    return out


def docs_title(doc):
    return doc["meta"].get("title") if isinstance(doc["meta"].get("title"), str) else derive_title(doc["body"]) or "Documentation"


def is_docs_layout(doc):
    return doc["meta"].get("layout") == "docs" or doc["meta"].get("theme") == "docs"


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
    component = next((item for item in component_definitions(options.get("doc")) if item.get("name") == node["type"] and isinstance(item.get("template"), str)), None)
    if component:
        return render_component_template(component, node, path, navigation, options)
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
        return render_table(node, path, navigation, options)
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
    if type_ == "grid":
        return wrap_layout_children("nodx-grid", node, path, navigation, options)
    if type_ == "columns":
        return wrap_layout_children("nodx-columns", node, path, navigation, options)
    if type_ == "frame":
        return wrap_layout_children("nodx-frame", node, path, navigation, options)
    if type_ == "page":
        return wrap_layout_children("nodx-page", node, path, navigation, options)
    if "-" in type_:
        return '<section' + html_attrs_without_class(node) + ' class="' + class_attr(node, "nodx-component nodx-component--fallback") + '" data-component="' + escape_attr(type_) + '"><p class="nodx-component__title">' + escape_html(type_) + " fallback</p>" + render_inlines(node["inlines"], options) + render_children(node, path, navigation, options) + "</section>"
    return wrap_children("div", node, path, navigation, options)


def render_children(node, path, navigation, options):
    return "".join(render_node(child, child_path(path, index), navigation, options) for index, child in enumerate(node["children"]))


def render_table(node, path, navigation, options):
    caption = node["attrs"].get("caption") or node["attrs"].get("title") or ""
    caption_html = "<caption>" + escape_html(caption) + "</caption>" if caption.strip() else ""
    return "<table" + html_attrs(node) + ">" + caption_html + render_children(node, path, navigation, options) + "</table>"


def wrap_layout_children(class_name, node, path, navigation, options):
    return "<div" + html_attrs_with_extra_class(node, class_name) + ">" + render_children(node, path, navigation, options) + "</div>"


def render_component_template(component, node, path, navigation, options):
    rendered = render_template_part(component["template"], component, node, path, navigation, options)
    children = render_inlines(node["inlines"], options) + render_children(node, path, navigation, options)
    return rendered.replace("<p><var>vars.children</var></p>", children).replace("<var>vars.children</var>", children)


def render_template_part(part, component, node, path, navigation, options):
    parsed = parse(substitute_template_vars(part, node, options.get("doc")))
    return "".join(render_node(child, path + ".template." + component["name"] + "." + str(index), navigation, options) for index, child in enumerate(parsed["body"]))


def substitute_template_vars(source, node, doc):
    def replace(match):
        key = match.group(1)
        if key == "children":
            return "{{children}}"
        if key.startswith("attrs."):
            return str(node["attrs"].get(key[6:], ""))
        if key.startswith("vars."):
            return str((doc or {}).get("meta", {}).get("vars", {}).get(key[5:], ""))
        return str(node["attrs"].get(key, ""))

    return re.sub(r"\{\{\s*([A-Za-z0-9_.-]+)\s*\}\}", replace, source)


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
    src = resolver(raw, node) if resolver else safe_image_url(raw, options.get("remoteAssets") is True)
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
    src = safe_media_url(raw, options.get("remoteAssets") is True)
    video = '<video controls src="' + escape_attr(src) + '"></video>' if node["type"] == "media" and src else ""
    text = text_resolver(raw) if node["type"] == "include" and text_resolver else (node["attrs"].get("alt") or node["type"]) + (" - " + src if src else "")
    return "<figure" + html_attrs(node) + ">" + video + '<div class="media-fallback">' + escape_html(text) + "</div>" + render_children(node, path, navigation, options) + "</figure>"


def render_inlines(inlines, options=None):
    options = options or {}
    out = []
    for item in inlines:
        type_ = item["type"]
        if type_ == "text":
            out.append(escape_html(item["text"]))
        elif type_ in ("strong", "em", "sub", "sup"):
            out.append("<" + type_ + ">" + render_inlines(item["children"], options) + "</" + type_ + ">")
        elif type_ == "mark":
            out.append("<mark" + inline_attrs(item.get("attrs")) + ">" + render_inlines(item["children"], options) + "</mark>")
        elif type_ == "strike":
            out.append("<s>" + render_inlines(item["children"], options) + "</s>")
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
    if node.get("styles"):
        out += ' style="' + escape_attr(style_attr(node["styles"])) + '"'
    if node["attrs"].get("lang"):
        out += ' lang="' + escape_attr(node["attrs"]["lang"]) + '"'
    if node["attrs"].get("dir") in ("ltr", "rtl", "auto"):
        out += ' dir="' + escape_attr(node["attrs"]["dir"]) + '"'
    if node["attrs"].get("title"):
        out += ' title="' + escape_attr(node["attrs"]["title"]) + '"'
    out += structural_attrs(node)
    return out


def html_attrs_without_class(node):
    out = html_id(node)
    if node["attrs"].get("lang"):
        out += ' lang="' + escape_attr(node["attrs"]["lang"]) + '"'
    if node["attrs"].get("dir") in ("ltr", "rtl", "auto"):
        out += ' dir="' + escape_attr(node["attrs"]["dir"]) + '"'
    if node["attrs"].get("title"):
        out += ' title="' + escape_attr(node["attrs"]["title"]) + '"'
    out += structural_attrs(node)
    return out


def html_attrs_with_extra_class(node, extra):
    out = html_id(node)
    out += ' class="' + escape_attr(" ".join([extra] + (node.get("classes") or []))) + '"'
    background_image = safe_background_image_css(node["attrs"].get("background"))
    if node.get("styles") or background_image:
        style = "; ".join(item for item in (style_attr(node.get("styles") or {}), background_image) if item)
        out += ' style="' + escape_attr(style) + '"'
    if node["attrs"].get("lang"):
        out += ' lang="' + escape_attr(node["attrs"]["lang"]) + '"'
    if node["attrs"].get("dir") in ("ltr", "rtl", "auto"):
        out += ' dir="' + escape_attr(node["attrs"]["dir"]) + '"'
    if node["attrs"].get("title"):
        out += ' title="' + escape_attr(node["attrs"]["title"]) + '"'
    out += structural_attrs(node)
    return out


def page_stylesheet(doc):
    page = doc.get("meta", {}).get("page")
    if not isinstance(page, dict):
        return ""
    rules = []
    bg = page.get("bg") or page.get("background-color")
    if isinstance(bg, str) and safe_quick_style_value(bg):
        rules.append("background-color:" + bg)
    color = page.get("color")
    if isinstance(color, str) and safe_quick_style_value(color):
        rules.append("color:" + color)
    bg_image = safe_background_image_css(page.get("background") or page.get("background-image"))
    if bg_image:
        rules.append(bg_image)
    return "body{" + ";".join(rules) + "}" if rules else ""


def safe_background_image_css(raw):
    if not isinstance(raw, str) or not safe_image_url(raw) or re.search(r"""['"()\\]""", raw):
        return ""
    return "background-image:url('" + raw + "')"


def safe_quick_style_value(value):
    return len(value) <= 240 and not any(ch in value for ch in "<>{};")


def structural_attrs(node):
    out = ""
    for key, value in node.get("attrs", {}).items():
        value = str(value)
        if safe_structural_attr(key, value):
            out += ' ' + key + '="' + escape_attr(value) + '"'
    return out


def safe_structural_attr(key, value):
    if key == "role":
        return re.match(r"^[A-Za-z0-9-]+$", value) is not None
    if key == "scope":
        return value in ("col", "row", "colgroup", "rowgroup")
    if key == "align":
        return value in ("left", "center", "right", "start", "end")
    if key == "valign":
        return value in ("top", "middle", "bottom", "baseline")
    if key in ("colspan", "rowspan"):
        return value.isdigit() and 1 <= int(value) <= 1000
    return (key.startswith("data-") or key.startswith("aria-")) and len(value) <= 240


def inline_attrs(attrs):
    if not attrs:
        return ""
    out = ' id="' + escape_attr(attrs["id"]) + '"' if attrs.get("id") else ""
    if attrs.get("classes"):
        out += ' class="' + escape_attr(" ".join(attrs["classes"])) + '"'
    if attrs.get("styles"):
        out += ' style="' + escape_attr(style_attr(attrs["styles"])) + '"'
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


def style_attr(styles):
    return "; ".join(key + ": " + value for key, value in styles.items())


def component_definitions(doc):
    components = (doc or {}).get("meta", {}).get("components")
    return [item for item in components if isinstance(item, dict) and isinstance(item.get("name"), str)] if isinstance(components, list) else []


def safe_link_url(raw):
    return raw if classify_uri(ReferenceKind.Link, raw)["ok"] else None


def safe_image_url(raw, remote_assets=False):
    return raw if classify_uri(ReferenceKind.Asset, raw, options={"remoteAssets": remote_assets})["ok"] else None


def safe_media_url(raw, remote_assets=False):
    return raw if classify_uri(ReferenceKind.MediaFallback, raw, options={"remoteAssets": remote_assets})["ok"] else None


def document_allows_remote_assets(doc):
    features = doc.get("meta", {}).get("features")
    if isinstance(features, dict) and features.get("remote-assets") is True:
        return True
    profiles = doc.get("meta", {}).get("profiles")
    if isinstance(profiles, dict):
        for key in ("requires", "optional"):
            if isinstance(profiles.get(key), list) and "remote-assets" in profiles[key]:
                return True
    return False


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
        type_ = item["type"]
        if type_ in ("text", "code"):
            out += item["text"]
        elif type_ == "math-inline":
            out += item["source"]
        elif type_ in ("strong", "em", "mark", "strike", "sub", "sup"):
            out += plain_inlines(item["children"])
        elif type_ == "link":
            out += plain_inlines(item["label"])
        elif type_ == "span":
            out += plain_inlines(item["children"])
        elif type_ == "var":
            out += "{{" + item["namespace"] + "." + item["name"] + "}}"
        elif type_ in ("ref", "footnote-ref", "citation-ref"):
            out += item["target"]
        elif type_ == "mention":
            out += "@" + item["kind"] + ":" + item["target"]
    return out


def standard_tokens():
    return 'html{font-family:system-ui}html[dir="rtl"]{direction:rtl}:root{--nodx-color-text:#1f2937;--nodx-color-muted:#4b5563;--nodx-color-bg:#ffffff;--nodx-color-primary:#0f766e;--nodx-color-accent:#b91c1c;--nodx-color-rule:#e5e7eb;--nodx-color-surface:transparent;--nodx-font-body:system-ui,-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;--nodx-font-heading:var(--nodx-font-body);--nodx-font-mono:ui-monospace,SFMono-Regular,Menlo,monospace;--nodx-page-margin:22mm;--nodx-line-height:1.6;--nodx-block-gap:1rem}'


def common_styles():
    return "h1,h2,h3,h4,h5,h6{font-family:var(--nodx-font-heading);line-height:1.25;color:#0f172a;margin-top:1.4em}p{margin:0 0 1em}pre{padding:12px;background:#f5f5f5;overflow:auto;border-radius:6px}code{font-family:var(--nodx-font-mono)}aside{border-inline-start:4px solid #b57f00;padding:8px 12px;background:#fff8e6}table{border-collapse:collapse;margin:0 0 1em}caption{text-align:start;font-weight:600;margin-bottom:.35em}td,th{border:1px solid #d1d5db;padding:6px 10px}thead th{background:#f3f4f6;text-align:start}figure{margin:1.5em 0}figcaption{font-size:.9em;color:var(--nodx-color-muted)}nav ol{padding-inline-start:1.5rem}nav strong{display:block;margin-bottom:.4em}.nodx-grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(12rem,1fr));gap:var(--nodx-block-gap);margin:0 0 1em}.nodx-columns{columns:2 18rem;column-gap:2rem;margin:0 0 1em}.nodx-frame{border:1px solid var(--nodx-color-rule,#e5e7eb);padding:1rem;margin:0 0 1em;border-radius:6px;background:var(--nodx-color-surface,transparent)}.nodx-blocked-link,.nodx-blocked-image{color:var(--nodx-color-accent);text-decoration:line-through}.nodx-blocked-link{cursor:not-allowed}.mention{font-variant:all-small-caps}.pagebreak{border:none;border-top:1px dashed #9ca3af;margin:2em 0}.math-inline{background:#f3f4f6;padding:1px 4px;border-radius:3px}.media-fallback{border:1px dashed #d1d5db;padding:12px;border-radius:6px;color:var(--nodx-color-muted)}"


def docs_styles():
    return 'body.nodx-docs-layout{font:16px/1.65 var(--nodx-font-body);color:var(--nodx-color-text);margin:0;display:grid;grid-template-columns:minmax(220px,280px) minmax(0,1fr) minmax(180px,240px);gap:0;min-height:100vh}.nodx-docs-sidebar,.nodx-docs-outline{position:sticky;top:0;height:100vh;overflow:auto;padding:24px 18px;border-color:#e5e7eb}.nodx-docs-sidebar{border-inline-end:1px solid #e5e7eb;background:#f8fafc}.nodx-docs-outline{border-inline-start:1px solid #e5e7eb;background:#fff}.nodx-docs-main{min-width:0;max-width:860px;width:100%;padding:32px 32px 64px;margin:0 auto}.nodx-docs-brand{display:block;font-weight:700;color:var(--nodx-color-text);text-decoration:none;margin-bottom:18px}.nodx-docs-layout nav ol{list-style:none;padding:0;margin:0}.nodx-docs-layout nav li{margin:2px 0}.nodx-docs-layout nav li[data-level="2"]{padding-inline-start:12px}.nodx-docs-layout nav li[data-level="3"],.nodx-docs-layout nav li[data-level="4"],.nodx-docs-layout nav li[data-level="5"],.nodx-docs-layout nav li[data-level="6"]{padding-inline-start:22px}.nodx-docs-layout nav a{display:block;color:#374151;text-decoration:none;border-radius:6px;padding:4px 6px}.nodx-docs-layout nav a:hover{background:#eef2ff;color:#111827}@media(max-width:920px){body.nodx-docs-layout{display:block}.nodx-docs-sidebar,.nodx-docs-outline{position:static;height:auto;border:0;border-bottom:1px solid #e5e7eb}.nodx-docs-outline{display:none}.nodx-docs-main{padding:24px 18px 48px}}'


def semantic_id(node):
    return " #" + node["id"] if node.get("id") else ""


def semantic_attrs(node):
    attrs = [f'{key}="{value}"' for key, value in sorted(node.get("attrs", {}).items()) if value not in ("", None) and key not in ("level", "header", "scope")]
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

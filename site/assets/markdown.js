/* Tiny Markdown renderer for the NODX docs site.
 *
 * Covers the constructs our docs actually use: ATX headings, fenced and
 * indented code, ordered/unordered/task lists, pipe tables, blockquotes,
 * horizontal rules, inline code/emphasis/links, autolinks, images, hard
 * line breaks, and HTML escaping.
 *
 * Intentionally NOT a full CommonMark implementation. If you need that,
 * vendor a real parser; this one is ~250 lines and predictable.
 */
(function (global) {
  "use strict";

  function escapeHtml(s) {
    return String(s)
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;")
      .replace(/'/g, "&#39;");
  }

  function escapeAttr(s) {
    return escapeHtml(s);
  }

  function slugify(text) {
    return String(text)
      .toLowerCase()
      .replace(/[^a-z0-9\s-]/g, "")
      .replace(/\s+/g, "-")
      .replace(/^-+|-+$/g, "")
      .slice(0, 64);
  }

  // ---- inline -----------------------------------------------------

  function renderInline(text) {
    // Inline code first so nothing inside it gets parsed.
    let out = "";
    let i = 0;
    const len = text.length;
    while (i < len) {
      const ch = text[i];

      // backtick code span
      if (ch === "`") {
        const end = text.indexOf("`", i + 1);
        if (end !== -1) {
          out += "<code>" + escapeHtml(text.slice(i + 1, end)) + "</code>";
          i = end + 1;
          continue;
        }
      }

      // images: ![alt](src)
      if (ch === "!" && text[i + 1] === "[") {
        const m = /^!\[([^\]]*)\]\(([^)\s]+)(?:\s+"([^"]*)")?\)/.exec(text.slice(i));
        if (m) {
          const alt = escapeAttr(m[1]);
          const src = escapeAttr(m[2]);
          const title = m[3] ? ' title="' + escapeAttr(m[3]) + '"' : "";
          out += '<img src="' + src + '" alt="' + alt + '"' + title + ' loading="lazy">';
          i += m[0].length;
          continue;
        }
      }

      // links: [label](href "title")
      if (ch === "[") {
        const m = /^\[([^\]]+)\]\(([^)\s]+)(?:\s+"([^"]*)")?\)/.exec(text.slice(i));
        if (m) {
          const label = renderInline(m[1]);
          let href = m[2];
          const title = m[3] ? ' title="' + escapeAttr(m[3]) + '"' : "";
          // Treat ".md" links as internal SPA hash routes.
          if (/^[^:/]+(?:\.md)?(?:#[^#]+)?$/.test(href) && !/^(mailto:|tel:|https?:)/.test(href)) {
            href = "#/" + href.replace(/^\.\//, "").replace(/\.md$/, "");
          } else {
            href = escapeAttr(href);
          }
          out += '<a href="' + href + '"' + title + '>' + label + "</a>";
          i += m[0].length;
          continue;
        }
      }

      // strong: **text**
      if (ch === "*" && text[i + 1] === "*") {
        const end = text.indexOf("**", i + 2);
        if (end !== -1) {
          out += "<strong>" + renderInline(text.slice(i + 2, end)) + "</strong>";
          i = end + 2;
          continue;
        }
      }
      // emphasis: *text*
      if (ch === "*") {
        const end = text.indexOf("*", i + 1);
        if (end !== -1) {
          out += "<em>" + renderInline(text.slice(i + 1, end)) + "</em>";
          i = end + 1;
          continue;
        }
      }
      // emphasis with underscore: _text_  (only when bounded)
      if (ch === "_" && /\s|^/.test(text[i - 1] || " ")) {
        const end = text.indexOf("_", i + 1);
        if (end !== -1) {
          out += "<em>" + renderInline(text.slice(i + 1, end)) + "</em>";
          i = end + 1;
          continue;
        }
      }

      // line break: two spaces at end of line followed by newline
      if (ch === "\n") {
        out += "\n";
        i += 1;
        continue;
      }

      // escape next char with backslash
      if (ch === "\\" && i + 1 < len) {
        out += escapeHtml(text[i + 1]);
        i += 2;
        continue;
      }

      out += escapeHtml(ch);
      i += 1;
    }
    return out;
  }

  // ---- block ------------------------------------------------------

  function parseTable(rows) {
    if (rows.length < 2) return null;
    const header = rows[0].trim();
    const sep    = rows[1].trim();
    if (!/^\|?[-: ]+\|[-: |]*$/.test(sep) && !/^\|?\s*:?-+:?\s*(\|\s*:?-+:?\s*)+\|?$/.test(sep)) {
      return null;
    }
    function splitRow(r) {
      return r.replace(/^\||\|$/g, "").split("|").map((s) => s.trim());
    }
    const headers = splitRow(header);
    const aligns  = splitRow(sep).map((cell) => {
      const left  = cell.startsWith(":");
      const right = cell.endsWith(":");
      if (left && right) return "center";
      if (right) return "right";
      if (left)  return "left";
      return null;
    });
    const body = rows.slice(2).map(splitRow);

    let html = '<table><thead><tr>';
    for (let i = 0; i < headers.length; i++) {
      const a = aligns[i] ? ' style="text-align:' + aligns[i] + '"' : "";
      html += '<th' + a + ">" + renderInline(headers[i]) + "</th>";
    }
    html += "</tr></thead>";
    if (body.length) {
      html += "<tbody>";
      for (const row of body) {
        html += "<tr>";
        for (let i = 0; i < row.length; i++) {
          const a = aligns[i] ? ' style="text-align:' + aligns[i] + '"' : "";
          html += "<td" + a + ">" + renderInline(row[i] || "") + "</td>";
        }
        html += "</tr>";
      }
      html += "</tbody>";
    }
    html += "</table>";
    return html;
  }

  function isListLine(line) {
    return /^\s*(?:[-*+]|\d+\.)\s+/.test(line);
  }
  function parseListItem(line) {
    const m = /^(\s*)(?:([-*+])|(\d+)\.)\s+(.*)$/.exec(line);
    if (!m) return null;
    let content = m[4];
    let checked = null;
    const taskMatch = /^\[( |x|X)\]\s+(.*)$/.exec(content);
    if (taskMatch) {
      checked = taskMatch[1].toLowerCase() === "x";
      content = taskMatch[2];
    }
    return {
      indent: m[1].length,
      ordered: !!m[3],
      content,
      checked,
    };
  }

  function render(md) {
    const src = String(md).replace(/\r\n?/g, "\n");
    const lines = src.split("\n");
    let i = 0;
    let html = "";

    while (i < lines.length) {
      const line = lines[i];

      // blank line
      if (line.trim() === "") { i++; continue; }

      // fenced code: ``` lang
      if (/^```/.test(line)) {
        const lang = line.replace(/^```\s*/, "").trim();
        const buf = [];
        i++;
        while (i < lines.length && !/^```\s*$/.test(lines[i])) {
          buf.push(lines[i]); i++;
        }
        if (i < lines.length) i++; // consume closing fence
        const cls = lang ? ' class="language-' + escapeAttr(lang) + '"' : "";
        html += "<pre><code" + cls + ">" + escapeHtml(buf.join("\n")) + "</code></pre>";
        continue;
      }

      // ATX heading
      const hMatch = /^(#{1,6})\s+(.*?)\s*#*$/.exec(line);
      if (hMatch) {
        const level = hMatch[1].length;
        const text  = hMatch[2];
        const id    = slugify(text);
        html += "<h" + level + ' id="' + id + '">' + renderInline(text)
             + ' <a class="anchor" href="#' + id + '" aria-hidden="true">#</a></h' + level + ">";
        i++; continue;
      }

      // horizontal rule
      if (/^(?:-{3,}|_{3,}|\*{3,})\s*$/.test(line)) {
        html += "<hr>"; i++; continue;
      }

      // table (heuristic: pipe in line, next line looks like a separator)
      if (line.includes("|") && i + 1 < lines.length && /^\s*\|?\s*:?-+:?\s*(\|\s*:?-+:?\s*)+\|?\s*$/.test(lines[i + 1])) {
        const rows = [line, lines[i + 1]];
        let j = i + 2;
        while (j < lines.length && lines[j].includes("|") && lines[j].trim() !== "") {
          rows.push(lines[j]); j++;
        }
        const tableHtml = parseTable(rows);
        if (tableHtml) {
          html += tableHtml;
          i = j;
          continue;
        }
      }

      // blockquote
      if (/^>\s?/.test(line)) {
        const buf = [];
        while (i < lines.length && /^>\s?/.test(lines[i])) {
          buf.push(lines[i].replace(/^>\s?/, ""));
          i++;
        }
        html += "<blockquote>" + render(buf.join("\n")) + "</blockquote>";
        continue;
      }

      // lists
      if (isListLine(line)) {
        const startOrdered = /^\s*\d+\.\s+/.test(line);
        const tag = startOrdered ? "ol" : "ul";
        let listHtml = "<" + tag + ">";
        while (i < lines.length && (isListLine(lines[i]) || (lines[i].trim() === "" && i + 1 < lines.length && isListLine(lines[i + 1])))) {
          if (lines[i].trim() === "") { i++; continue; }
          const item = parseListItem(lines[i]); i++;
          const itemBuf = [item.content];
          while (i < lines.length && !isListLine(lines[i]) && lines[i].trim() !== "" && /^\s{2,}/.test(lines[i])) {
            itemBuf.push(lines[i].replace(/^\s{2}/, ""));
            i++;
          }
          let inner = renderInline(itemBuf.join("\n"));
          let prefix = "";
          if (item.checked !== null) {
            prefix = '<input type="checkbox" disabled' + (item.checked ? " checked" : "") + "> ";
          }
          listHtml += "<li>" + prefix + inner + "</li>";
        }
        listHtml += "</" + tag + ">";
        html += listHtml;
        continue;
      }

      // paragraph: gather until blank line or a block-starting line
      const buf = [line];
      i++;
      while (i < lines.length && lines[i].trim() !== ""
             && !/^```/.test(lines[i])
             && !/^(#{1,6})\s+/.test(lines[i])
             && !/^(?:-{3,}|_{3,}|\*{3,})\s*$/.test(lines[i])
             && !/^>\s?/.test(lines[i])
             && !isListLine(lines[i])
             && !(lines[i].includes("|") && i + 1 < lines.length && /^\s*\|?\s*:?-+:?\s*(\|\s*:?-+:?\s*)+\|?\s*$/.test(lines[i + 1]))) {
        buf.push(lines[i]); i++;
      }
      html += "<p>" + renderInline(buf.join("\n")) + "</p>";
    }

    return html;
  }

  global.NodxMarkdown = { render, slugify, escapeHtml };
})(typeof window !== "undefined" ? window : globalThis);

/* SPA router for the NODX docs site.
 * Hash-based routing so the site works on GitHub Pages without server
 * rewrites. Pages are static Markdown files in `pages/` relative to the
 * site root; they are fetched on demand and rendered with NodxMarkdown.
 */
(function () {
  "use strict";

  const NAV = [
    {
      title: "Get started",
      items: [
        { route: "home",                  label: "Welcome" },
        { route: "guide/01-quickstart",   label: "Quickstart" },
        { route: "guide/02-syntax-tour",  label: "Syntax tour" },
        { route: "guide/03-authoring",    label: "Authoring" },
        { route: "guide/04-packaging",    label: "Packaging" },
        { route: "guide/05-cli",          label: "CLI" },
      ],
    },
    {
      title: "Reference",
      items: [
        { route: "reference/blocks",       label: "Blocks" },
        { route: "reference/inline",       label: "Inline syntax" },
        { route: "reference/attributes",   label: "Attributes" },
        { route: "reference/ast",          label: "Canonical AST" },
        { route: "reference/diagnostics",  label: "Diagnostics" },
        { route: "reference/profiles",     label: "Profiles" },
        { route: "reference/themes",       label: "Themes & styling" },
        { route: "reference/limits",       label: "Resource limits" },
        { route: "reference/ncp",          label: "NCP projection" },
        { route: "reference/conformance",  label: "Conformance" },
      ],
    },
    {
      title: "Cookbook",
      items: [
        { route: "cookbook/index", label: "Recipes" },
      ],
    },
    {
      title: "Internals",
      items: [
        { route: "internals/architecture",        label: "Architecture" },
        { route: "internals/scalability",         label: "Scalability" },
        { route: "internals/streaming-evolution", label: "Streaming (proposal)" },
        { route: "internals/security-model",      label: "Security model" },
      ],
    },
    {
      title: "Specification",
      items: [
        { route: "spec/NODX-RFC-0001", label: "NODX 1.0 RFC" },
        { route: "spec/SECURITY",      label: "Security policy" },
      ],
    },
  ];

  // ------------------- DOM helpers ----------------------

  const $  = (sel) => document.querySelector(sel);
  const $$ = (sel) => Array.from(document.querySelectorAll(sel));

  // ------------------- Sidebar build --------------------

  function buildSidebar() {
    const nav = $("#sidebar-nav");
    const frag = document.createDocumentFragment();
    for (const section of NAV) {
      const h = document.createElement("h3");
      h.textContent = section.title;
      frag.appendChild(h);
      const ol = document.createElement("ol");
      for (const item of section.items) {
        const li = document.createElement("li");
        const a  = document.createElement("a");
        a.href = "#/" + item.route;
        a.textContent = item.label;
        a.dataset.route = item.route;
        li.appendChild(a);
        ol.appendChild(li);
      }
      frag.appendChild(ol);
    }
    nav.appendChild(frag);
  }

  // ------------------- Routing --------------------------

  function flatRoutes() {
    const out = [];
    for (const section of NAV) for (const item of section.items) out.push(item);
    return out;
  }

  function findRoute(route) {
    return flatRoutes().find((r) => r.route === route) || null;
  }

  function adjacent(route) {
    const flat = flatRoutes();
    const idx = flat.findIndex((r) => r.route === route);
    if (idx === -1) return { prev: null, next: null };
    return {
      prev: idx > 0 ? flat[idx - 1] : null,
      next: idx < flat.length - 1 ? flat[idx + 1] : null,
    };
  }

  function markActive(route) {
    $$("#sidebar-nav a").forEach((a) => a.classList.toggle("is-active", a.dataset.route === route));
  }

  async function loadMarkdown(route) {
    if (route === "home") return null;
    const url = "pages/" + route + ".md";
    const resp = await fetch(url, { cache: "no-cache" });
    if (!resp.ok) throw new Error("Failed to load " + url + " (" + resp.status + ")");
    return await resp.text();
  }

  function renderHome() {
    return `
<section class="hero">
  <h1>Structured documents for people, tools, and AI agents.</h1>
  <p class="hero__lede">
    NODX is a text-first format with the readability of Markdown, the
    rigor of a canonical AST, and the safety of a fail-closed processor.
    Render to HTML, project to JSON, package with assets, sign a release —
    same source, same bytes, every time.
  </p>
  <div class="hero__cta">
    <a class="cta-primary" href="#/guide/01-quickstart">Get started →</a>
    <a class="cta-ghost"   href="#/reference/blocks">Browse the reference</a>
  </div>
</section>

<section class="feature-grid">
  <div class="feature">
    <h3>Canonical, byte-stable AST</h3>
    <p>Two conformant parsers produce identical JSON for the same input. Diffs work.</p>
  </div>
  <div class="feature">
    <h3>Fail-closed safety</h3>
    <p>No scripts, no remote fetch, no surprises. Hostile inputs hit a documented diagnostic.</p>
  </div>
  <div class="feature">
    <h3>Six built-in profiles</h3>
    <p><code>plain</code>, <code>core</code>, <code>rich</code>, <code>style</code>, <code>package</code>, <code>agent-read</code>. Pick what you need.</p>
  </div>
  <div class="feature">
    <h3>NCP for agents</h3>
    <p>Per-node hashes, semantic chunks, stable paths. Built for retrieval pipelines.</p>
  </div>
  <div class="feature">
    <h3>Packaged ZIP container</h3>
    <p>Source + assets + components + themes, hashed and verifiable in one file.</p>
  </div>
  <div class="feature">
    <h3>Scales to a 2 000-page book</h3>
    <p>Linear in input size. Measured at <strong>0.16 s, 80 MB RAM</strong> for HTML.</p>
  </div>
</section>

<section class="code-tabs">
  <h2>Look at it</h2>
  <pre><code class="language-nodx">---
title: Quarterly report
theme: web
---

# Quarterly report #q1

| Metric  | Q4 2025 | Q1 2026 | Delta |
|---------|--------:|--------:|------:|
| Revenue |     90K |    120K |  +33% |
| Costs   |     70K |     80K |  +14% |

::note {type="info"}
We hit the revenue target two weeks early.
::</code></pre>

  <h2>Render it</h2>
  <pre><code class="language-sh">$ nodx html report.nodx &gt; report.html
$ nodx validate report.nodx --format json
$ nodx ncp report.nodx --mode chunks &gt; report.ndjson</code></pre>

  <h2>Trust it</h2>
  <p>
    The reference implementation is ~1 MB of Rust spread over 12 focused crates,
    with <code>#![forbid(unsafe_code)]</code> in every one. The conformance bundle
    enforces byte-equality between the Rust and JavaScript parsers across every
    committed fixture.
  </p>
</section>`;
  }

  function buildOutline(rootEl) {
    const outline = $("#outline");
    outline.innerHTML = "";
    const headings = rootEl.querySelectorAll("h2, h3, h4");
    if (headings.length === 0) return;
    const frag = document.createDocumentFragment();
    headings.forEach((h) => {
      const li = document.createElement("li");
      li.dataset.level = h.tagName.slice(1);
      const a = document.createElement("a");
      a.href = "#" + h.id;
      a.textContent = h.textContent.replace(/\s*#\s*$/, "");
      a.dataset.targetId = h.id;
      li.appendChild(a);
      frag.appendChild(li);
    });
    outline.appendChild(frag);
  }

  function attachOutlineScrollSpy() {
    const links = $$("#outline a");
    if (links.length === 0) return;
    const map = new Map();
    links.forEach((a) => {
      const target = document.getElementById(a.dataset.targetId);
      if (target) map.set(target, a);
    });
    const observer = new IntersectionObserver((entries) => {
      entries.forEach((entry) => {
        const a = map.get(entry.target);
        if (!a) return;
        if (entry.isIntersecting) {
          links.forEach((l) => l.classList.remove("is-active"));
          a.classList.add("is-active");
        }
      });
    }, { rootMargin: "-30% 0px -55% 0px", threshold: 0 });
    map.forEach((_, t) => observer.observe(t));
  }

  function renderPager(route) {
    const prevEl = $("#pager-prev");
    const nextEl = $("#pager-next");
    const { prev, next } = adjacent(route);
    if (prev) {
      prevEl.hidden = false;
      prevEl.href = "#/" + prev.route;
      prevEl.querySelector("span").textContent = prev.label;
    } else {
      prevEl.hidden = true;
    }
    if (next) {
      nextEl.hidden = false;
      nextEl.href = "#/" + next.route;
      nextEl.querySelector("span").textContent = next.label;
    } else {
      nextEl.hidden = true;
    }
  }

  function setGithubLinks() {
    const repoBase = window.NODX_REPO_BASE || "https://github.com/";
    $$("[data-github]").forEach((a) => { a.href = repoBase; });
  }

  async function navigate(route) {
    document.body.classList.remove("nav-open");
    $("[data-nav-toggle]").setAttribute("aria-expanded", "false");
    const article = $("#article");
    article.setAttribute("aria-busy", "true");

    let html;
    let title = "NODX — Structured documents";
    try {
      if (route === "home") {
        html = renderHome();
      } else {
        const md = await loadMarkdown(route);
        html = NodxMarkdown.render(md);
        const m = /<h1[^>]*>([\s\S]*?)<\/h1>/.exec(html);
        if (m) {
          const stripped = m[1].replace(/<[^>]+>/g, "").trim();
          if (stripped) title = stripped + " — NODX";
        }
      }
    } catch (err) {
      html = "<h1>Not found</h1><p>The page <code>" + NodxMarkdown.escapeHtml(route) + "</code> could not be loaded.</p><p><a href=\"#/home\">Back to the welcome page</a></p>";
    }

    article.innerHTML = html;
    article.setAttribute("aria-busy", "false");
    document.title = title;

    buildOutline(article);
    attachOutlineScrollSpy();
    renderPager(route);
    markActive(route);

    // Scroll either to the anchor on the route, or to the top.
    const hash = location.hash.includes("#", 2) ? location.hash.split("#").slice(2).join("#") : "";
    if (hash) {
      const target = document.getElementById(hash);
      if (target) { target.scrollIntoView(); return; }
    }
    window.scrollTo({ top: 0, left: 0, behavior: "instant" });
  }

  function parseRoute() {
    let h = location.hash || "#/home";
    if (h.startsWith("#/")) h = h.slice(2);
    else if (h.startsWith("#")) h = h.slice(1);
    if (!h) return "home";
    const onlyHash = h.match(/^([^#]*)(?:#.*)?$/);
    return (onlyHash ? onlyHash[1] : h) || "home";
  }

  // ------------------- Search ---------------------------

  let searchIndex = null;

  async function loadSearchIndex() {
    if (searchIndex) return searchIndex;
    try {
      const resp = await fetch("data/search-index.json", { cache: "no-cache" });
      if (!resp.ok) return [];
      searchIndex = await resp.json();
    } catch (e) {
      searchIndex = [];
    }
    return searchIndex;
  }

  function rankHits(query, items) {
    const q = query.toLowerCase().trim();
    if (!q) return [];
    const terms = q.split(/\s+/).filter(Boolean);
    return items
      .map((it) => {
        const hay = (it.title + " " + (it.text || "")).toLowerCase();
        let score = 0;
        for (const t of terms) {
          if (it.title.toLowerCase().includes(t)) score += 5;
          if (hay.includes(t)) score += 1;
        }
        return { item: it, score };
      })
      .filter((x) => x.score > 0)
      .sort((a, b) => b.score - a.score)
      .slice(0, 20)
      .map((x) => x.item);
  }

  function snippet(text, query) {
    const q = query.toLowerCase().trim().split(/\s+/)[0] || "";
    const idx = (text || "").toLowerCase().indexOf(q);
    if (idx === -1) return (text || "").slice(0, 120);
    const start = Math.max(0, idx - 30);
    return (start > 0 ? "…" : "") + text.slice(start, start + 140) + "…";
  }

  function renderSearchResults(query, hits) {
    const list = $("#search-results");
    if (!query.trim()) { list.hidden = true; list.innerHTML = ""; return; }
    if (hits.length === 0) {
      list.hidden = false;
      list.innerHTML = '<li><a aria-disabled="true">No results</a></li>';
      return;
    }
    list.hidden = false;
    list.innerHTML = "";
    for (const hit of hits) {
      const li = document.createElement("li");
      const a  = document.createElement("a");
      a.href = "#/" + hit.route;
      a.innerHTML = NodxMarkdown.escapeHtml(hit.title)
        + '<small>' + NodxMarkdown.escapeHtml(snippet(hit.text || "", query)) + '</small>';
      li.appendChild(a);
      list.appendChild(li);
    }
  }

  // ------------------- Theme toggle ----------------------

  function applySavedTheme() {
    const t = localStorage.getItem("nodx-theme");
    if (t === "dark" || t === "light") document.documentElement.setAttribute("data-theme", t);
  }

  function toggleTheme() {
    const cur = document.documentElement.getAttribute("data-theme");
    const prefersDark = matchMedia("(prefers-color-scheme: dark)").matches;
    const isDark = cur ? cur === "dark" : prefersDark;
    const next = isDark ? "light" : "dark";
    document.documentElement.setAttribute("data-theme", next);
    localStorage.setItem("nodx-theme", next);
  }

  // ------------------- Boot ------------------------------

  function boot() {
    buildSidebar();
    setGithubLinks();
    applySavedTheme();

    window.addEventListener("hashchange", () => navigate(parseRoute()));
    navigate(parseRoute());

    $("[data-theme-toggle]").addEventListener("click", toggleTheme);
    $("[data-nav-toggle]").addEventListener("click", function () {
      document.body.classList.toggle("nav-open");
      this.setAttribute("aria-expanded", document.body.classList.contains("nav-open") ? "true" : "false");
    });

    const input = $("#search");
    const list  = $("#search-results");
    input.addEventListener("input", async function () {
      const idx = await loadSearchIndex();
      renderSearchResults(this.value, rankHits(this.value, idx));
    });
    input.addEventListener("blur", function () {
      // close after a short delay to allow clicks on results to register
      setTimeout(() => { list.hidden = true; }, 120);
    });
    input.addEventListener("focus", function () {
      if (this.value) renderSearchResults(this.value, rankHits(this.value, searchIndex || []));
    });

    // Smooth navigation through "/" key
    document.addEventListener("keydown", function (e) {
      if (e.key === "/" && document.activeElement !== input && !e.metaKey && !e.ctrlKey && !e.altKey) {
        e.preventDefault();
        input.focus();
        input.select();
      }
    });
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", boot);
  } else {
    boot();
  }
})();

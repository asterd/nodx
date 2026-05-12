const quickPlain = `Plain NODX can be just text.

Blank lines split paragraphs. Inline code like \`schema: nodx/1.0\`, strong text like **stable contract**, and links like [the RFC](../../NODX-RFC-0001.md) work without front matter.

# Optional heading #plain-heading

- Plain authoring
- Implicit core profile
- Safe HTML rendering`;

const yamlStyle = `---
schema: nodx/1.0
title: YAML style sample
theme: web
profiles:
  requires:
    - core
    - style
---
# YAML style sample #yaml-style

:::style {format="yaml"}
h1:
  color: var(--nodx-color-primary)
  border-bottom: 2px solid var(--nodx-color-primary)
  padding-bottom: 8px
aside:
  background: #ecfdf5
  border-inline-start-color: #0f766e
:::style

::note {type="tip"}
This style block uses the YAML authoring form accepted by the validator.
::note`;

const themeSample = `---
schema: nodx/1.0
title: Theme tokens and overrides
theme: web
profiles:
  requires:
    - core
    - style
---
# Theme tokens and overrides #themes

Use the theme selector in the toolbar to preview the same source with \`base\`,
\`web\`, \`print\`, \`presentation\`, \`plain\`, and \`none\`.

:::style
:root {
  --nodx-color-primary: #1d4ed8;
  --nodx-color-accent: #b91c1c;
  --nodx-font-heading: Georgia, serif;
  --nodx-page-margin: 18mm;
}
h1 { color: var(--nodx-color-primary); }
aside { border-inline-start-color: var(--nodx-color-accent); }
::: style

::note {type="tip"}
Theme CSS comes from the JS renderer. This document only overrides safe NODX
tokens and a few safe selectors through \`::style\`.
:: note

::pagebreak
:: pagebreak

## Print section #print-section

Switch to \`print\` and enable Pages to see the paged host approximation.`;

const layoutSample = `---
schema: nodx/1.0
title: Layout, table, media and forms
theme: web
profiles:
  requires:
    - core
    - rich
---
# Layout sample #layout

::toc {title="Index" depth="3"}
::toc

::section {.app-shell}
::section {.side-nav}
## Local nav #local-nav

::toc {title="Local contents" scope="local" depth="3"}
::toc
::section

::section {.content-panel}
## Dashboard #dashboard

::section {.feature-grid}
:::note {type="tip"}
The renderer keeps unknown layout classes visible instead of dropping them.
:::note

:::note
Cards, grids, sidebars and page breaks are ordinary blocks with classes.
:::note
::section

| Area | Status | Owner |
| --- | --- | --- |
| Parser | aligned | core |
| Web playground | active | apps/web |
| Editors | syntax | editors |

::form
:::field {name="release" label="Release" value="1.0 playground"}
:::field
:::field {name="profile" label="Profile" value="core+rich+style"}
:::field
::form
::section
::section`;

const inlineCoverage = `# Inline coverage #inline

Strong **text**, emphasis *text*, mark ==highlight==, subscript H~2~O, superscript x^2^, code \`node.type\`, variable {{reviewer}}, reference @[inline], citation [@rfc-0001], footnote [^note-1], mention @{person:ada}, math $$E=mc^2$$, span [localized text]{lang="it" dir="ltr"}, and [safe link](https://example.com){title="safe"}.

::bibliography
:::citation-entry {#rfc-0001}
NODX RFC 0001.
:::citation-entry
::bibliography

::note {#note-1}
Footnote target represented as a normal block for preview purposes.
::note`;

const variablesSample = `---
schema: nodx/1.0
title: Variables preview
theme: web
vars:
  reviewer: Ada
  status: approved
  amount: 120K
profiles:
  requires:
    - core
---
# Variables preview #variables

Reviewer: {{reviewer}}.
Status: {{status}}.
Amount: {{amount}}.
Title from metadata: {{meta.title}}.

Edit the Variables tab to apply different runtime values without changing the source.`;

const longDocument = `---
schema: nodx/1.0
title: Long document stress sample
language: en
theme: print
profiles:
  requires:
    - core
    - rich
    - style
---
# Long document stress sample #long

::toc {title="Document index" depth="3"}
::toc

This opening paragraph is intentionally very long and has no manual page break.
It exists to exercise the host paged preview when content overflows a selected
format. ${Array.from({ length: 55 }, (_, index) => `Automatic pagination sentence ${index + 1} keeps flowing through the same paragraph so the preview must split it without an explicit pagebreak.`).join(" ")}

${Array.from({ length: 18 }, (_, section) => {
  const n = section + 1;
  return `## Section ${n} #section-${n}

This section is intentionally repetitive so the playground can exercise long source editing, outline generation, internal links, validation, and paged preview. It includes stable anchors, paragraph flow, a note block, and a compact table.

::note {type="${n % 3 === 0 ? "tip" : "note"}"}
Checkpoint ${n}: the renderer should keep scrolling, page simulation, and AST inspection responsive.
::note

| Metric | Value | Notes |
| --- | --- | --- |
| Section | ${n} | stable id section-${n} |
| Paragraphs | 3 | long document sample |
| Page break | ${n % 5 === 0 ? "after section" : "no"} | preview only |

${n % 5 === 0 ? "::pagebreak\n::pagebreak" : ""}`;
}).join("\n\n")}`;

const packageEntry = `---
schema: nodx/1.0
title: Generated package sample
theme: web
profiles:
  requires:
    - core
    - rich
    - package
components:
  - name: approval-card
    fallback: children
---
# Generated package sample #package

::figure
:::image {src="assets/pipeline.svg" alt="Reference pipeline diagram"}
:::image
:::caption
This package is generated in the browser as a stored ZIP and read back through the same package reader used for real .nodx bundles.
:::caption
::figure

::include {src="partials/appendix.nodx"}
::include

::approval-card {#loaded-component status="pending" fallback="children"}
This block is rendered by a custom renderer loaded from
\`components/playground-components.nodc\` inside the generated package.
::approval-card`;

export const examples = [
  {
    group: "Inline",
    id: "plain-inline",
    label: "Plain: implicit document",
    kind: "inline",
    text: quickPlain,
  },
  {
    group: "Inline",
    id: "inline-coverage",
    label: "Inline coverage",
    kind: "inline",
    text: inlineCoverage,
  },
  {
    group: "Inline",
    id: "variables-preview",
    label: "Variables preview",
    kind: "inline",
    text: variablesSample,
  },
  {
    group: "Inline",
    id: "style-yaml",
    label: "Style: YAML syntax",
    kind: "inline",
    text: yamlStyle,
  },
  {
    group: "Inline",
    id: "theme-overrides",
    label: "Themes and token overrides",
    kind: "inline",
    text: themeSample,
  },
  {
    group: "Inline",
    id: "layout-table-form",
    label: "Layout, table and form",
    kind: "inline",
    text: layoutSample,
  },
  {
    group: "Inline",
    id: "long-document",
    label: "Long document stress",
    kind: "inline",
    text: longDocument,
  },
  {
    group: "Repository examples",
    id: "repo-minimal",
    label: "Minimal",
    kind: "file",
    path: "../../examples/minimal.nodx",
  },
  {
    group: "Repository examples",
    id: "repo-plain",
    label: "Plain",
    kind: "file",
    path: "../../examples/plain.nodx",
  },
  {
    group: "Repository examples",
    id: "repo-rich",
    label: "Rich demo",
    kind: "file",
    path: "../../examples/rich-demo.nodx",
  },
  {
    group: "Repository examples",
    id: "repo-agent",
    label: "Agent workflow",
    kind: "file",
    path: "../../examples/agent-workflow.nodx",
  },
  {
    group: "Repository examples",
    id: "repo-showcase",
    label: "Extended showcase",
    kind: "file",
    path: "../../examples/extended-showcase.nodx",
  },
  {
    group: "Repository examples",
    id: "repo-navigation",
    label: "Navigation sidebar",
    kind: "file",
    path: "../../examples/navigation-sidebar.nodx",
  },
  {
    group: "Repository examples",
    id: "repo-layout-fonts",
    label: "Layout and fonts",
    kind: "file",
    path: "../../examples/layout-fonts.nodx",
  },
  {
    group: "Repository examples",
    id: "repo-inline-styles-components",
    label: "Inline styles and components",
    kind: "file",
    path: "../../examples/inline-styles-components.nodx",
  },
  {
    group: "Repository examples",
    id: "repo-docs-layout",
    label: "Docs layout",
    kind: "file",
    path: "../../examples/docs-layout.nodx",
  },
  {
    group: "Repository examples",
    id: "repo-typography",
    label: "Typography",
    kind: "file",
    path: "../../examples/typography.nodx",
  },
  {
    group: "Pagination and print",
    id: "repo-pagination",
    label: "Pagination",
    kind: "file",
    path: "../../examples/pagination.nodx",
  },
  {
    group: "Pagination and print",
    id: "repo-multipage",
    label: "Multi-page report",
    kind: "file",
    path: "../../examples/multi-page-report.nodx",
  },
  {
    group: "Pagination and print",
    id: "repo-print-portrait",
    label: "Print: A4 portrait",
    kind: "file",
    path: "../../examples/print/print-portrait.nodx",
  },
  {
    group: "Pagination and print",
    id: "repo-print-landscape",
    label: "Print: A3 landscape",
    kind: "file",
    path: "../../examples/print/print-landscape.nodx",
  },
  {
    group: "Internationalization",
    id: "repo-arabic",
    label: "Arabic RTL",
    kind: "file",
    path: "../../examples/i18n/arabic-rtl.nodx",
  },
  {
    group: "Internationalization",
    id: "repo-chinese",
    label: "Chinese CJK",
    kind: "file",
    path: "../../examples/i18n/chinese-cjk.nodx",
  },
  {
    group: "Internationalization",
    id: "repo-mixed",
    label: "Mixed scripts",
    kind: "file",
    path: "../../examples/i18n/mixed-scripts.nodx",
  },
  {
    group: "Package ZIP",
    id: "repo-bundled",
    label: "Stored ZIP: repository bundle",
    kind: "file",
    path: "../../examples/extended-showcase-bundled.nodx",
  },
  {
    group: "Package ZIP",
    id: "generated-package",
    label: "Stored ZIP: generated package",
    kind: "package",
    files: {
      "mimetype": "application/nodx+zip",
      "manifest.yaml": `schema: nodx-package/1.0
entry: content/document.nodx
entries:
  - path: content/document.nodx
  - path: styles/package.nods
  - path: assets/pipeline.svg
  - path: components/playground-components.nodc
  - path: partials/appendix.nodx
`,
      "content/document.nodx": packageEntry,
      "styles/package.nods": `.nodx-component[data-component="approval-card"] {
  border-color: #b45309;
  background-color: #fff7ed;
}
`,
      "components/playground-components.nodc": `{
  "schema": "nodx-components/0.1",
  "components": [
    {
      "name": "approval-card",
      "version": "0.1.0",
      "fallback": "children",
      "render": {
        "kind": "callout",
        "title": "Loaded approval-card renderer",
        "class": "approval-card"
      }
    }
  ]
}
`,
      "partials/appendix.nodx": "# Appendix\n\nPackage readers can expose included files and assets to higher-level tools.",
      "assets/pipeline.svg": `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 520 180" role="img" aria-label="Pipeline">
  <rect width="520" height="180" rx="12" fill="#f8fafc"/>
  <g fill="#0f766e" font-family="system-ui" font-size="18" font-weight="700">
    <text x="44" y="96">Source</text>
    <text x="222" y="96">Package</text>
    <text x="404" y="96">Render</text>
  </g>
  <g stroke="#2563eb" stroke-width="4" fill="none" stroke-linecap="round">
    <path d="M130 90h70"/>
    <path d="M310 90h70"/>
  </g>
  <g fill="#dbeafe" stroke="#2563eb" stroke-width="2">
    <rect x="26" y="52" width="110" height="72" rx="10"/>
    <rect x="204" y="52" width="120" height="72" rx="10"/>
    <rect x="386" y="52" width="108" height="72" rx="10"/>
  </g>
</svg>`,
    },
  },
];

export function findExample(id) {
  return examples.find((example) => example.id === id) ?? examples[0];
}

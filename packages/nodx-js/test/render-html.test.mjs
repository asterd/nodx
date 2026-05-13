import assert from "node:assert/strict";
import { test } from "node:test";

import { parse, renderFragment, renderHtml } from "../parser.mjs";

test("renders class suffix on styled spans", () => {
  const doc = parse("Text [[status text]].status-pill.success.");
  assert.match(renderFragment(doc), /<span class="status-pill success">status text<\/span>/);
});

test("renders table captions, cell spans, and layout blocks", () => {
  const doc = parse(`:::table {caption="Revenue"}
:::row
:::cell {header="true" colspan=2 align="center"}
Total
:::
:::
:::

:::grid {gap="2rem"}
:::frame {bg="#f8fafc"}
A
:::
:::
`);
  const html = renderHtml(doc);
  assert.match(html, /<caption>Revenue<\/caption>/);
  assert.match(html, /colspan="2"/);
  assert.match(html, /align="center"/);
  assert.match(html, /class="nodx-grid"/);
  assert.match(html, /class="nodx-frame"/);
  assert.match(html, /gap: 2rem/);
  assert.match(html, /background-color: #f8fafc/);
});

test("renders quick mark, strike, and page backgrounds", () => {
  const doc = parse(`---
schema: nodx/1.0
page:
  bg: "#101827"
  color: "#f8fafc"
  background: "assets/bg.png"
---

==Marked=={bg="#ffe08a" color="#111827"} and ~~removed~~.

:::page {bg="#ffffff" background="assets/page.png"}
Page body.
:::
`);
  const html = renderHtml(doc);
  assert.match(html, /body\{background-color:#101827;color:#f8fafc;background-image:url\('assets\/bg.png'\)\}/);
  assert.match(html, /<mark style="background-color: #ffe08a; color: #111827">Marked<\/mark>/);
  assert.match(html, /<s>removed<\/s>/);
  assert.match(html, /class="nodx-page"/);
  assert.match(html, /background-image:url\(&#x27;assets\/page.png&#x27;\)/);
});

test("renders styled remote media with native video fallback only", () => {
  const doc = parse(`---
schema: nodx/1.0
features:
  remote-assets: true
---

:::image {src="https://example.com/image.png" alt="Remote image" width="320px" border="1px solid #cbd5e1"}
:::

:::media {src="https://example.com/video.mp4" alt="Remote video" width="480px" margin="1rem 0"}
:::media-fallback
Fallback text.
:::
:::
`);
  const html = renderFragment(doc);
  assert.match(html, /<img style="width: 320px; border: 1px solid #cbd5e1" src="https:\/\/example.com\/image.png" alt="Remote image">/);
  assert.match(html, /<figure style="width: 480px; margin: 1rem 0"><video controls src="https:\/\/example.com\/video.mp4"><p>Fallback text\.<\/p><\/video><\/figure>/);
  assert.doesNotMatch(html, /<div class="media-fallback">/);
});

test("renders docs layout as a two-navigation document shell", () => {
  const doc = parse(`---
schema: nodx/1.0
title: Docs
theme: docs
layout: docs
---
# Docs #docs

## Install #install

### CLI #cli

# Reference #reference

## Blocks #blocks
`);
  const html = renderHtml(doc);
  assert.match(html, /<body class="nodx-docs-layout">/);
  assert.match(html, /class="nodx-docs-sidebar"/);
  assert.match(html, /class="nodx-docs-main"/);
  assert.match(html, /class="nodx-docs-outline"/);
  const sidebar = html.match(/<aside class="nodx-docs-sidebar">([\s\S]*?)<\/aside>/)?.[1] ?? "";
  const outline = html.match(/<aside class="nodx-docs-outline">([\s\S]*?)<\/aside>/)?.[1] ?? "";
  assert.match(sidebar, /href="#docs"/);
  assert.match(sidebar, /href="#reference"/);
  assert.doesNotMatch(sidebar, /href="#install"/);
  assert.match(outline, /href="#install"/);
  assert.match(outline, /href="#cli"/);
  assert.match(outline, /href="#blocks"/);
});

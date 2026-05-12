import assert from "node:assert/strict";
import { test } from "node:test";

import { parse, renderFragment, renderHtml } from "../parser.mjs";

test("renders class suffix on styled spans", () => {
  const doc = parse("Text [[status text]].status-pill.success.");
  assert.match(renderFragment(doc), /<span class="status-pill success">status text<\/span>/);
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
`);
  const html = renderHtml(doc);
  assert.match(html, /<body class="nodx-docs-layout">/);
  assert.match(html, /class="nodx-docs-sidebar"/);
  assert.match(html, /class="nodx-docs-main"/);
  assert.match(html, /class="nodx-docs-outline"/);
  assert.match(html, /href="#install"/);
  assert.match(html, /href="#cli"/);
});

import assert from "node:assert/strict";
import { test } from "node:test";

import { parse } from "../parser.mjs";

// Text-side resource limits parity with `nodx_core::parse_str_with_limits`.
// See packages/nodx-py/tests/test_text_resource_limits.py for the Py twin.

test("source byte limit emits NODX-E012", () => {
  const huge = "A".repeat(100);
  const doc = parse(huge, {
    sourceBytes: 10,
    lineLength: 1024 * 1024,
    frontMatterBytes: 64 * 1024,
  });
  const fatal = doc.diagnostics.find((d) => d.code === "NODX-E012" && d.severity === "fatal");
  assert.ok(fatal, "expected NODX-E012 fatal for oversized source");
  assert.equal(doc.body.length, 0);
});

test("line length limit emits NODX-E012", () => {
  const longline = "A".repeat(100);
  const doc = parse(longline, {
    sourceBytes: 1024 * 1024,
    lineLength: 10,
    frontMatterBytes: 64 * 1024,
  });
  assert.ok(doc.diagnostics.some((d) => d.code === "NODX-E012"));
});

test("front matter byte limit emits NODX-E012", () => {
  let src = "---\n";
  for (let i = 0; i < 50; i += 1) src += "k: v\n";
  src += "---\n\nbody\n";
  const doc = parse(src, {
    sourceBytes: 1024 * 1024,
    lineLength: 1024 * 1024,
    frontMatterBytes: 10,
  });
  assert.ok(doc.diagnostics.some((d) => d.code === "NODX-E012"));
  assert.equal(doc.body.length, 0);
});

test("normal input within limits parses cleanly", () => {
  const doc = parse("# Hello\n");
  assert.equal(doc.schema, "nodx/1.0");
  assert.ok(doc.body.length);
  assert.ok(!doc.diagnostics.some((d) => d.severity === "fatal"));
});

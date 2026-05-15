import assert from "node:assert/strict";
import { test } from "node:test";
import { parse } from "../src/index.mjs";

// Mirrors `crates/nodx-core/src/tests.rs` W030..W035 coverage. The Rust
// suite is authoritative for behaviour; this file checks the JS twin
// agrees on which codes fire and which severity they carry. Byte-stable
// parity is enforced separately by `scripts/run_conformance.sh`.

function codes(doc) {
  return doc.diagnostics.map((d) => d.code);
}

test("W030 setext heading emits a warning", () => {
  const doc = parse("Title\n=====\n");
  assert.ok(codes(doc).includes("NODX-W030"));
  const w = doc.diagnostics.find((d) => d.code === "NODX-W030");
  assert.equal(w.severity, "warning");
  assert.equal(w.line, 2);
});

test("W031 indented code block fires once per contiguous run", () => {
  const doc = parse("    fn main() {}\n    println!();\n\n    again\n");
  const hits = doc.diagnostics.filter((d) => d.code === "NODX-W031");
  assert.equal(hits.length, 2);
  assert.equal(hits[0].line, 1);
  assert.equal(hits[1].line, 4);
});

test("W032 inline image emits a warning", () => {
  const doc = parse("See ![logo](logo.png) here.\n");
  assert.ok(codes(doc).includes("NODX-W032"));
});

test("W033 link reference definition emits a warning", () => {
  const doc = parse("[ref]: https://example.test\n");
  assert.ok(codes(doc).includes("NODX-W033"));
});

test("W033 inline link reference emits a warning", () => {
  const doc = parse("Use [label][ref] here.\n");
  assert.ok(codes(doc).includes("NODX-W033"));
});

test("W034 footnote definition emits a warning", () => {
  const doc = parse("[^fn]: footnote text.\n");
  assert.ok(codes(doc).includes("NODX-W034"));
});

test("W035 HTML entity references emit warnings", () => {
  const doc = parse("Use &amp; and &#x76; and &#33; here.\n");
  const hits = doc.diagnostics.filter((d) => d.code === "NODX-W035");
  assert.equal(hits.length, 3);
  assert.ok(hits.every((d) => d.severity === "warning"));
});

test("CommonMark warnings are non-fatal", () => {
  const doc = parse("Title\n=====\n\n    code\n\n[ref]: x\n");
  assert.ok(!doc.diagnostics.some((d) => d.severity === "fatal"));
  assert.ok(!doc.diagnostics.some((d) => d.severity === "error"));
});

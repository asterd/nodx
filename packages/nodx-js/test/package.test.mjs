import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

import { openStoredPackage, packageEntryText, parse } from "../parser.mjs";

test("opens stored packaged NODX example", () => {
  const bytes = readFileSync(new URL("../../../examples/extended-showcase-bundled.nodx", import.meta.url));
  const pkg = openStoredPackage(bytes);
  assert.equal(pkg.entryPath, "content/document.nodx");
  assert.ok(pkg.files.has("assets/reference-pipeline.svg"));
  assert.match(packageEntryText(bytes), /Extended Showcase/);
  const doc = parse(packageEntryText(bytes));
  assert.equal(doc.schema, "nodx/1.0");
});

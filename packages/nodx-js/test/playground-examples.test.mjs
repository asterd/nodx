import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

import { examples } from "../../../apps/web/examples.js";
import { isPackagedNodx, parse, validate } from "../parser.mjs";

const root = new URL("../../..", import.meta.url);

test("playground examples are file-backed and valid", () => {
  assert.ok(examples.length > 0);
  for (const example of examples) {
    assert.equal(example.kind, "file", example.id);
    assert.equal(typeof example.path, "string", example.id);
    assert.match(example.path, /^\.\.\/\.\.\/examples\/playground\//, example.id);
    const path = example.path.replace(/^\.\.\/\.\.\//, "");
    const bytes = readFileSync(new URL(path, root));
    if (isPackagedNodx(new Uint8Array(bytes))) continue;
    const diagnostics = validate(parse(bytes.toString("utf8")));
    assert.deepEqual(
      diagnostics.filter((item) => item.severity !== "warning"),
      [],
      example.id,
    );
  }
});

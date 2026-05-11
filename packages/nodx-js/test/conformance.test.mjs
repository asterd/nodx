import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { canonicalJson, diagnosticsJson, ncpJson, parse, validate } from "../src/index.mjs";

const root = new URL("../../..", import.meta.url).pathname;
const rust = join(root, "target/debug/nodx");

for (const file of positiveFixtures()) {
  const source = readFileSync(join(root, file), "utf8");
  const doc = parse(source);
  const rustAst = execFileSync(rust, ["ast", file], { cwd: root, encoding: "utf8" });
  assert.equal(canonicalJson(doc) + "\n", rustAst, file + " AST");
  const rustNcp = execFileSync(rust, ["ncp", file], { cwd: root, encoding: "utf8" });
  assert.equal(ncpJson(doc) + "\n", rustNcp, file + " NCP");
}

for (const name of readdirSync(join(root, "spec/tests/golden")).filter((item) => item.endsWith(".diagnostics.json"))) {
  const fixture = "spec/tests/negative/" + name.replace(".diagnostics.json", ".nodx");
  const source = readFileSync(join(root, fixture), "utf8");
  const expected = readFileSync(join(root, "spec/tests/golden", name), "utf8").trimEnd();
  assert.equal(diagnosticsJson(validate(parse(source))), expected, fixture + " diagnostics");
}

function positiveFixtures() {
  return [
    ...nodxFiles("spec/tests/conformance"),
    ...nodxFiles("spec/tests/ncp"),
    ...nodxFiles("spec/tests/navigation"),
    ...nodxFiles("spec/tests/rendering"),
  ];
}

function nodxFiles(dir) {
  return readdirSync(join(root, dir))
    .filter((name) => name.endsWith(".nodx"))
    .sort()
    .map((name) => dir + "/" + name);
}

#!/usr/bin/env node
import { readFileSync } from "node:fs";
import { canonicalJson, parse } from "../parser.mjs";

const [, , command, file] = process.argv;
if (command !== "ast" || !file) {
  console.error("usage: nodx-js ast <file.nodx>");
  process.exit(2);
}
process.stdout.write(canonicalJson(parse(readFileSync(file, "utf8"))) + "\n");

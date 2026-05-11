#!/usr/bin/env node
import { readFileSync } from "node:fs";
import { canonicalJson, diagnosticsJson, exitCodeFor, ncpJson, parse, validate } from "../src/index.mjs";

const [, , command, file] = process.argv;
if (!["ast", "ncp", "diagnostics"].includes(command) || !file) {
  console.error("usage: nodx-js <ast|ncp|diagnostics> <file.nodx>");
  process.exit(2);
}

const doc = parse(readFileSync(file, "utf8"));
if (command === "ast") process.stdout.write(canonicalJson(doc) + "\n");
else if (command === "ncp") process.stdout.write(ncpJson(doc) + "\n");
else {
  const diagnostics = validate(doc);
  process.stdout.write(diagnosticsJson(diagnostics) + "\n");
  process.exit(exitCodeFor(diagnostics));
}

import assert from "node:assert/strict";
import { sha256Base64Url } from "../src/bytes.mjs";
import { canonicalJson, ncpJson, parse } from "../src/index.mjs";

const doc = parse("# Title {#title}\n\n:::toc {#toc depth=\"1\"}\n:::\n");
const numericDoc = parse("---\nschema: nodx/1.0\nvars:\n  release: 1.0\n---\n# Title\n");

assert.equal(sha256Base64Url("abc"), "sha256-ungWv48Bz-pBQUDeXa4iI7ADYaOWF3qctBD_YfIAFa0");
assert.equal(canonicalJson(doc), "{\"body\":[{\"attrs\":{\"level\":\"1\"},\"children\":[],\"classes\":[],\"id\":\"title\",\"inlines\":[{\"text\":\"Title\",\"type\":\"text\"}],\"text\":null,\"type\":\"heading\"},{\"attrs\":{\"depth\":\"1\"},\"children\":[],\"classes\":[],\"id\":\"toc\",\"inlines\":[],\"text\":null,\"type\":\"toc\"}],\"meta\":{\"dir\":\"auto\",\"language\":\"und\",\"schema\":\"nodx/1.0\",\"type\":\"document\"},\"schema\":\"nodx/1.0\"}");
assert.equal(canonicalJson(numericDoc), "{\"body\":[{\"attrs\":{\"level\":\"1\"},\"children\":[],\"classes\":[],\"id\":null,\"inlines\":[{\"text\":\"Title\",\"type\":\"text\"}],\"text\":null,\"type\":\"heading\"}],\"meta\":{\"dir\":\"auto\",\"language\":\"und\",\"schema\":\"nodx/1.0\",\"type\":\"document\",\"vars\":{\"release\":1}},\"schema\":\"nodx/1.0\"}");
assert.match(ncpJson(doc), /"navigationEntries":\[\{"id":"title","level":1,"path":"0","title":"Title"\}\]/);

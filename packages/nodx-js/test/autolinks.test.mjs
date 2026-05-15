import assert from "node:assert/strict";
import { canonicalJson, parse } from "../src/index.mjs";

// PR3: `<scheme:body>` and `<email>` produce Inline::Link identical to the
// `[label](target)` form. Safety lives in `nodx-url` (validate/render path),
// so the parser stays policy-free.

const httpsDoc = parse("# T\n\nSee <https://example.com>.\n");
const jsonHttps = canonicalJson(httpsDoc);
assert.match(jsonHttps, /"type":"link"/, "https autolink should be a link");
assert.match(jsonHttps, /"target":"https:\/\/example\.com"/);

const emailDoc = parse("# T\n\nMail <foo@bar.com>.\n");
const jsonEmail = canonicalJson(emailDoc);
assert.match(jsonEmail, /"target":"mailto:foo@bar\.com"/, "bare email autolink should be mailto-prefixed");

const mailtoDoc = parse("# T\n\nMail <mailto:foo@bar.com>.\n");
const jsonMailto = canonicalJson(mailtoDoc);
assert.match(jsonMailto, /"target":"mailto:foo@bar\.com"/);
// No double prefix: the body already had mailto:, so target stays verbatim.
assert.ok(!jsonMailto.includes("mailto:mailto:"));

const spaceDoc = parse("# T\n\nNot <not a url> here.\n");
const jsonSpace = canonicalJson(spaceDoc);
assert.ok(!jsonSpace.includes('"type":"link"'), "spaces inside <…> disqualify autolink");

const jsDoc = parse("# T\n\nBad <javascript:alert(1)> stays a link node.\n");
const jsonJs = canonicalJson(jsDoc);
// Parser is policy-free; the validator/renderer rejects it.
assert.match(jsonJs, /"target":"javascript:alert\(1\)"/);

const multiDoc = parse("# T\n\nHit <https://a.example> and <mailto:b@example.com>.\n");
const jsonMulti = canonicalJson(multiDoc);
assert.match(jsonMulti, /"target":"https:\/\/a\.example"/);
assert.match(jsonMulti, /"target":"mailto:b@example\.com"/);

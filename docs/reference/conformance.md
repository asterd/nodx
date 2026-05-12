# Conformance

A conformant NODX implementation produces, for every input the reference
parser accepts:

- the same canonical AST, byte for byte;
- the same NCP projection, byte for byte (per mode);
- the same set of diagnostic codes, in the same order.

That is the entire claim. The HTML output is *not* part of the
conformance contract — themes evolve and renderers differ.

## The package

[`spec/conformance/v1.0/`](../../spec/conformance/v1.0/) is the canonical
test bundle for the 1.0 contract:

```
spec/conformance/v1.0/
├─ manifest.json        ← which fixtures, which expected outputs
├─ README.md            ← release notes for this version of the bundle
├─ fixtures/
│  ├─ minimal.nodx
│  ├─ rich-web.nodx
│  ├─ lite-syntax.nodx
│  └─ invalid-required-profile.nodx
└─ expected/
   ├─ minimal.ast.json
   ├─ minimal.ncp.json
   ├─ minimal.diagnostics.json
   ├─ minimal.html
   └─ … one file per (fixture, format) pair
```

`manifest.json` lists every fixture and every expected output. A
conformance run reads the manifest, parses each fixture, and compares the
implementation's output against the expected bytes.

## Running the reference suite

```sh
sh scripts/run_conformance.sh
```

Output:

```
ok spec/conformance/v1.0/fixtures/minimal.nodx
ok spec/conformance/v1.0/fixtures/rich-web.nodx
ok spec/conformance/v1.0/fixtures/lite-syntax.nodx
ok examples/agent-workflow.nodx
…
```

The script runs both the Rust and JavaScript reference parsers on every
fixture, plus every committed example, and diffs each `ast`, `ncp`, and
`diagnostics` output against the expected bytes. A non-zero exit code
means a divergence; the offending file is named in the error.

It writes a structured report to `target/conformance-report.json` for CI
consumption.

## Running it from a third-party implementation

A typical integration test for an alternative implementation:

1. For each fixture in `manifest.json`, invoke your parser.
2. For each expected output it lists, compare your bytes against the
   expected bytes (after a `jq -S .` sort on JSON to remove key ordering
   noise if your library does not sort already — the reference does).
3. Treat any byte-level mismatch as a fail.

There is no "lenient mode" of conformance. The whole point is byte
stability.

## What "passes" means

Passing the conformance bundle does *not* mean:

- Your implementation has the same internal data structures.
- Your implementation has the same performance profile.
- Your implementation has the same error message text in human-readable
  output.

It does mean:

- The same documents produce the same canonical AST.
- The same documents produce the same NCP.
- The same documents produce the same set of `NODX-Exxx` codes in source
  order.

That is enough that downstream tools — search indexes, agents, content
diff systems — can rely on either implementation interchangeably.

## Updating the package

The conformance bundle moves on its own cadence, separately from the
implementation. A new bundle version (`v1.1`, `v2.0`) is published when:

- The canonical AST shape changes (additive, not renaming).
- A new diagnostic code is introduced and exercised by a fixture.
- A new profile is added.

Older bundles do not disappear; they remain in `spec/conformance/<old>/`
for as long as the contract claim of "1.0" is supported.

## Reporting a conformance bug

If you find an input where the Rust and JavaScript reference parsers
disagree, that is the bug we most want to hear about. Open an issue with:

- the fixture file;
- the Rust output (`cargo run -p nodx -- ast …`);
- the JavaScript output (`node packages/nodx-js/bin/cli.mjs ast …`);
- the diff you observed.

Conformance divergences are blockers, not enhancements.

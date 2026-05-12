# Cookbook

Patterns that come up over and over. Each one is a complete, runnable
recipe.

## Render to HTML in a build pipeline

```yaml
# .github/workflows/render.yml
name: Render NODX
on:
  push:
    paths: ["docs/**/*.nodx"]
jobs:
  render:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - run: cargo build --release -p nodx
      - run: |
          mkdir -p site
          for doc in docs/**/*.nodx; do
            out=site/$(basename "$doc" .nodx).html
            target/release/nodx html "$doc" > "$out"
          done
      - uses: actions/upload-pages-artifact@v3
        with:
          path: site
```

## Validate in CI without rendering

```sh
fail=0
for doc in $(git ls-files '*.nodx'); do
  if ! target/release/nodx validate "$doc" --format json | tee -a out.json | jq -e '.[] | select(.severity == "fatal" or .severity == "error")' >/dev/null; then
    :  # no errors
  else
    echo "::error file=$doc::Validation failed"
    fail=1
  fi
done
exit $fail
```

## Embed a document inside an existing page

Use `theme: none` to skip the stylesheet — your host page provides it:

```nodx
---
title: Embed me
theme: none
---

# Hello
```

Then extract the body from the rendered HTML:

```sh
nodx html embed.nodx \
  | sed -n '/<main>/,/<\/main>/p' \
  > _includes/embedded.html
```

## Diff two documents semantically

Two documents that mean the same thing produce the same canonical AST,
even if their source whitespace differs.

```sh
nodx ast a.nodx | jq -S . > a.ast.json
nodx ast b.nodx | jq -S . > b.ast.json
diff -u a.ast.json b.ast.json
```

If the AST diff is empty, the documents are equivalent.

## Hash a document for caching

```sh
nodx ast doc.nodx | sha256sum
```

The hash is stable across runs of the same binary version, so it is a
fine cache key. For change tracking at sub-document granularity, walk the
NCP tree and key off each node's `sha256` field.

## Generate a search index

```sh
nodx ncp doc.nodx \
  | jq -c '.. | objects | select(has("type") and has("path")) | {id, path, type, text, sha256}' \
  >> search-index.ndjson
```

Each line is a JSON record suitable for direct upload to Meilisearch,
OpenSearch, or any vector store after embedding.

## Sign a release artifact

A release pipeline that signs its NODX outputs:

```sh
# Build the canonical AST hash
hash=$(nodx ast doc.nodx | sha256sum | cut -d' ' -f1)

# Sign it (using your own JWS tool)
echo -n "$hash" \
  | step crypto jws sign --kid release-2026 --key release.pem \
  > doc.nodx.sig

# Bundle into a package
cp doc.nodx.sig examples/release-bundle/
python3 scripts/build_package.py examples/release-bundle
```

The receiving side:

```sh
nodx package verify release-bundle.nodx --signature release-bundle.nodx.sig
```

## Render a multi-page report to PDF (preview)

```sh
nodx export pdf examples/showcase-web.nodx -o report.pdf
```

The exporter writes a paged HTML pipeline and a loss report next to the
output (`report.pdf.loss.json`). Inspect the loss report before shipping
the PDF — it lists every NODX construct that did not fully survive the
conversion.

## Bundle assets and components in a package

```text
my-doc/
├─ mimetype
├─ manifest.yaml
├─ doc.nodx
├─ assets/
│  └─ cover.png
└─ components/
   └─ approval-card.nodx
```

```sh
python3 scripts/build_package.py my-doc
nodx package inspect my-doc.nodx
nodx package verify my-doc.nodx
```

The result is a single `my-doc.nodx` file that contains everything the
renderer needs and is safe to ship across systems.

## Use a document as an agent's source of truth

```python
import json, subprocess

ncp = json.loads(subprocess.check_output(["nodx", "ncp", "doc.nodx"]))

def walk(nodes):
    for node in nodes:
        yield node
        yield from walk(node["children"])

for node in walk(ncp["nodes"]):
    if node["type"] == "heading":
        continue
    embed(node["id"] or node["path"], node["text"], hash_=node["sha256"])
```

Re-running the script after the document changes will produce new hashes
only for the nodes that actually changed. Skip the rest.

## Sanitize a stylesheet before shipping it

```sh
nodx validate doc-with-style.nodx --format json \
  | jq '.[] | select(.code == "NODX-E027")'
```

Every rule that did not survive the audit shows up as `NODX-E027`. Use
the output to clean the source before publishing.

## Run the conformance sweep before tagging a release

```sh
sh scripts/run_conformance.sh \
  && cargo test --workspace --release \
  && echo "ready to tag"
```

If both pass, the implementation matches the spec and every regression
test holds. Tag the release.

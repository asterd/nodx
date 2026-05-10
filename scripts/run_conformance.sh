#!/usr/bin/env sh
set -eu

cargo build -q -p nodx
python3 scripts/build_package.py >/dev/null

for file in spec/tests/conformance/*.nodx examples/*.nodx examples/i18n/*.nodx examples/print/*.nodx; do
  if [ "$(target/debug/nodx inspect "$file" | sed -n '1p')" = "format: packaged-nodx" ]; then
    continue
  fi
  rust_out="$(target/debug/nodx ast "$file")"
  js_out="$(node packages/nodx-js/bin/nodx-js.mjs ast "$file")"
  if [ "$rust_out" != "$js_out" ]; then
    echo "AST mismatch: $file" >&2
    echo "rust: $rust_out" >&2
    echo "js:   $js_out" >&2
    exit 1
  fi
  target/debug/nodx html "$file" >/dev/null
  target/debug/nodx tui "$file" >/dev/null
  target/debug/nodx ncp "$file" >/dev/null
  echo "ok $file"
done

target/debug/nodx inspect examples/extended-showcase-bundled.nodx >/dev/null
target/debug/nodx ast examples/extended-showcase-bundled.nodx >/dev/null
target/debug/nodx html examples/extended-showcase-bundled.nodx >/dev/null
target/debug/nodx tui examples/extended-showcase-bundled.nodx >/dev/null
echo "ok examples/extended-showcase-bundled.nodx"

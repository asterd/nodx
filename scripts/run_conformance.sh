#!/usr/bin/env sh
set -eu

report="target/conformance-report.json"
tmp_dir="target/conformance"

cargo build -q -p nodx
python3 scripts/build_package.py >/dev/null
mkdir -p "$tmp_dir"

printf '{"fixtures":[' > "$report"
first=1

record() {
  if [ "$first" -eq 0 ]; then
    printf ',' >> "$report"
  fi
  first=0
  printf '{"file":"%s","ast":"%s","ncp":"%s"}' "$1" "$2" "$3" >> "$report"
}

for file in spec/tests/conformance/*.nodx spec/tests/ncp/*.nodx spec/tests/navigation/*.nodx spec/tests/rendering/*.nodx examples/*.nodx examples/i18n/*.nodx examples/print/*.nodx; do
  if [ "$(target/debug/nodx inspect "$file" | sed -n '1p')" = "format: packaged-nodx" ]; then
    record "$file" "skipped-packaged" "skipped-packaged"
    continue
  fi
  safe_name="$(printf '%s' "$file" | tr '/.' '__')"
  rust_ast="$tmp_dir/$safe_name.rust.ast.json"
  js_ast="$tmp_dir/$safe_name.js.ast.json"
  rust_ncp="$tmp_dir/$safe_name.rust.ncp.json"
  js_ncp="$tmp_dir/$safe_name.js.ncp.json"
  target/debug/nodx ast "$file" > "$rust_ast"
  node packages/nodx-js/bin/nodx-js.mjs ast "$file" > "$js_ast"
  if ! cmp -s "$rust_ast" "$js_ast"; then
    echo "AST mismatch: $file" >&2
    echo "rust: $(cat "$rust_ast")" >&2
    echo "js:   $(cat "$js_ast")" >&2
    printf ']}\n' >> "$report"
    exit 1
  fi
  target/debug/nodx ncp "$file" > "$rust_ncp"
  node packages/nodx-js/bin/nodx-js.mjs ncp "$file" > "$js_ncp"
  if ! cmp -s "$rust_ncp" "$js_ncp"; then
    echo "NCP mismatch: $file" >&2
    echo "rust: $(cat "$rust_ncp")" >&2
    echo "js:   $(cat "$js_ncp")" >&2
    printf ']}\n' >> "$report"
    exit 1
  fi
  record "$file" "ok" "ok"
  target/debug/nodx html "$file" >/dev/null
  target/debug/nodx tui "$file" >/dev/null
  echo "ok $file"
done

target/debug/nodx inspect examples/extended-showcase-bundled.nodx >/dev/null
target/debug/nodx ast examples/extended-showcase-bundled.nodx >/dev/null
target/debug/nodx html examples/extended-showcase-bundled.nodx >/dev/null
target/debug/nodx tui examples/extended-showcase-bundled.nodx >/dev/null
echo "ok examples/extended-showcase-bundled.nodx"

printf ']}\n' >> "$report"

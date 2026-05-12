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
  printf '{"file":"%s","ast":"%s","ncp":"%s","diagnostics":"%s"}' "$1" "$2" "$3" "$4" >> "$report"
}

run_capture() {
  out="$1"
  shift
  set +e
  "$@" > "$out"
  RUN_STATUS=$?
  set -e
  return 0
}

compare_fixture() {
  file="$1"
  safe_name="$(printf '%s' "$file" | tr '/.' '__')"
  rust_ast="$tmp_dir/$safe_name.rust.ast.json"
  js_ast="$tmp_dir/$safe_name.js.ast.json"
  rust_ncp="$tmp_dir/$safe_name.rust.ncp.json"
  js_ncp="$tmp_dir/$safe_name.js.ncp.json"
  rust_diag="$tmp_dir/$safe_name.rust.diag.json"
  js_diag="$tmp_dir/$safe_name.js.diag.json"

  run_capture "$rust_ast" target/debug/nodx ast "$file"
  rust_status=$RUN_STATUS
  run_capture "$js_ast" node packages/nodx-js/bin/nodx-js.mjs ast "$file"
  js_status=$RUN_STATUS
  if [ "$rust_status" -ne 0 ] || [ "$js_status" -ne 0 ]; then
    echo "AST command failed unexpectedly: $file rust=$rust_status js=$js_status" >&2
    printf ']}\n' >> "$report"
    exit 1
  fi
  if ! cmp -s "$rust_ast" "$js_ast"; then
    echo "AST mismatch: $file" >&2
    diff "$rust_ast" "$js_ast" >&2 || true
    printf ']}\n' >> "$report"
    exit 1
  fi
  run_capture "$rust_ncp" target/debug/nodx ncp "$file"
  rust_status=$RUN_STATUS
  run_capture "$js_ncp" node packages/nodx-js/bin/nodx-js.mjs ncp "$file"
  js_status=$RUN_STATUS
  if [ "$rust_status" -ne 0 ] || [ "$js_status" -ne 0 ]; then
    echo "NCP command failed unexpectedly: $file rust=$rust_status js=$js_status" >&2
    printf ']}\n' >> "$report"
    exit 1
  fi
  if ! cmp -s "$rust_ncp" "$js_ncp"; then
    echo "NCP mismatch: $file" >&2
    diff "$rust_ncp" "$js_ncp" >&2 || true
    printf ']}\n' >> "$report"
    exit 1
  fi
  run_capture "$rust_diag" target/debug/nodx diagnostics "$file" --format json
  rust_status=$RUN_STATUS
  run_capture "$js_diag" node packages/nodx-js/bin/nodx-js.mjs diagnostics "$file"
  js_status=$RUN_STATUS
  if [ "$rust_status" -ne "$js_status" ]; then
    echo "Diagnostics exit mismatch: $file rust=$rust_status js=$js_status" >&2
    printf ']}\n' >> "$report"
    exit 1
  fi
  if ! cmp -s "$rust_diag" "$js_diag"; then
    echo "Diagnostics mismatch: $file" >&2
    diff "$rust_diag" "$js_diag" >&2 || true
    printf ']}\n' >> "$report"
    exit 1
  fi
  record "$file" "ok" "ok" "ok"
  target/debug/nodx html "$file" >/dev/null || true
  target/debug/nodx tui "$file" >/dev/null || true
  echo "ok $file"
}

compare_negative_fixture() {
  file="$1"
  safe_name="$(printf '%s' "$file" | tr '/.' '__')"
  rust_diag="$tmp_dir/$safe_name.rust.diag.json"
  js_diag="$tmp_dir/$safe_name.js.diag.json"

  run_capture "$rust_diag" target/debug/nodx diagnostics "$file" --format json
  rust_status=$RUN_STATUS
  run_capture "$js_diag" node packages/nodx-js/bin/nodx-js.mjs diagnostics "$file"
  js_status=$RUN_STATUS
  if [ "$rust_status" -ne "$js_status" ]; then
    echo "Negative diagnostics exit mismatch: $file rust=$rust_status js=$js_status" >&2
    printf ']}\n' >> "$report"
    exit 1
  fi
  if [ "$(cat "$rust_diag")" = "[]" ]; then
    echo "Negative fixture produced no diagnostics: $file" >&2
    printf ']}\n' >> "$report"
    exit 1
  fi
  if ! cmp -s "$rust_diag" "$js_diag"; then
    echo "Negative diagnostics mismatch: $file" >&2
    diff "$rust_diag" "$js_diag" >&2 || true
    printf ']}\n' >> "$report"
    exit 1
  fi
  record "$file" "skipped-negative" "skipped-negative" "ok"
  echo "ok $file"
}

for file in spec/tests/conformance/*.nodx spec/tests/ncp/*.nodx spec/tests/navigation/*.nodx spec/tests/rendering/*.nodx spec/conformance/v1.0/fixtures/minimal.nodx spec/conformance/v1.0/fixtures/rich-web.nodx spec/conformance/v1.0/fixtures/lite-syntax.nodx examples/*.nodx examples/i18n/*.nodx examples/print/*.nodx; do
  if [ "$(target/debug/nodx inspect "$file" | sed -n '1p')" = "format: packaged-nodx" ]; then
    record "$file" "skipped-packaged" "skipped-packaged" "skipped-packaged"
    continue
  fi
  compare_fixture "$file"
done

for file in spec/tests/negative/*.nodx; do
  compare_negative_fixture "$file"
done

# Packaged sample: exercise the Rust package reader end-to-end without JS parity.
target/debug/nodx package inspect examples/extended-showcase-bundled.nodx >/dev/null
target/debug/nodx package verify examples/extended-showcase-bundled.nodx >/dev/null
target/debug/nodx ast examples/extended-showcase-bundled.nodx >/dev/null
target/debug/nodx html examples/extended-showcase-bundled.nodx >/dev/null
target/debug/nodx tui examples/extended-showcase-bundled.nodx >/dev/null
echo "ok examples/extended-showcase-bundled.nodx"

printf ']}\n' >> "$report"

#!/usr/bin/env sh
set -eu

cargo build -q -p nodx

root="spec/conformance/v1.0"
tmp="target/conformance-package"
mkdir -p "$tmp"

check_output() {
  name="$1"
  command="$2"
  expected="$3"
  actual="$tmp/$name"
  $command > "$actual"
  cmp -s "$actual" "$root/$expected"
}

check_output minimal.ast "target/debug/nodx ast $root/fixtures/minimal.nodx" expected/minimal.ast.json
check_output minimal.ncp "target/debug/nodx ncp $root/fixtures/minimal.nodx" expected/minimal.ncp.json
check_output minimal.diagnostics "target/debug/nodx diagnostics $root/fixtures/minimal.nodx --format json" expected/minimal.diagnostics.json
check_output minimal.html "target/debug/nodx html $root/fixtures/minimal.nodx" expected/minimal.html

check_output rich-web.ast "target/debug/nodx ast $root/fixtures/rich-web.nodx" expected/rich-web.ast.json
check_output rich-web.ncp "target/debug/nodx ncp $root/fixtures/rich-web.nodx" expected/rich-web.ncp.json
check_output rich-web.diagnostics "target/debug/nodx diagnostics $root/fixtures/rich-web.nodx --format json" expected/rich-web.diagnostics.json
check_output rich-web.html "target/debug/nodx html $root/fixtures/rich-web.nodx" expected/rich-web.html

check_output lite-syntax.ast "target/debug/nodx ast $root/fixtures/lite-syntax.nodx" expected/lite-syntax.ast.json
check_output lite-syntax.ncp "target/debug/nodx ncp $root/fixtures/lite-syntax.nodx" expected/lite-syntax.ncp.json
check_output lite-syntax.diagnostics "target/debug/nodx diagnostics $root/fixtures/lite-syntax.nodx --format json" expected/lite-syntax.diagnostics.json
check_output lite-syntax.html "target/debug/nodx html $root/fixtures/lite-syntax.nodx" expected/lite-syntax.html

check_output thematic-break.ast "target/debug/nodx ast $root/fixtures/thematic-break.nodx" expected/thematic-break.ast.json
check_output thematic-break.ncp "target/debug/nodx ncp $root/fixtures/thematic-break.nodx" expected/thematic-break.ncp.json
check_output thematic-break.diagnostics "target/debug/nodx diagnostics $root/fixtures/thematic-break.nodx --format json" expected/thematic-break.diagnostics.json
check_output thematic-break.html "target/debug/nodx html $root/fixtures/thematic-break.nodx" expected/thematic-break.html

if target/debug/nodx validate "$root/fixtures/invalid-required-profile.nodx" --format json > "$tmp/invalid-required-profile.diagnostics.json"; then
  echo "invalid-required-profile unexpectedly passed" >&2
  exit 1
else
  code="$?"
  if [ "$code" -ne 3 ]; then
    echo "invalid-required-profile exit code was $code, expected 3" >&2
    exit 1
  fi
fi
cmp -s "$tmp/invalid-required-profile.diagnostics.json" "$root/expected/invalid-required-profile.diagnostics.json"

echo "conformance package ok"

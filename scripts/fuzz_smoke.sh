#!/usr/bin/env sh
set -eu

runs="${NODX_FUZZ_RUNS:-5000}"
targets="${NODX_FUZZ_TARGETS:-parse_bytes front_matter block_parser inline_parser attrs url package nods ncp navigation}"
cargo_fuzz="${NODX_FUZZ_CARGO:-cargo +nightly fuzz}"

if ! $cargo_fuzz --help >/dev/null 2>&1; then
  echo "cargo-fuzz with a nightly Rust toolchain is required." >&2
  echo "Install with: rustup toolchain install nightly && cargo install cargo-fuzz" >&2
  exit 1
fi

for target in $targets; do
  echo "fuzz smoke: $target ($runs runs)"
  $cargo_fuzz run "$target" -- -runs="$runs"
done

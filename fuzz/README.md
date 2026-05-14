# NODX Fuzzing

These targets exercise public parser, validator-adjacent, package, style, URL,
NCP, and navigation boundaries. They are intentionally kept outside the main
Cargo workspace so normal `rtk cargo test` does not require `cargo-fuzz`.

## Setup

Install `cargo-fuzz` in the local Rust toolchain:

```sh
rtk rustup toolchain install nightly
rtk cargo install cargo-fuzz
```

## Targets

| Target | Boundary |
|---|---|
| `parse_bytes` | UTF-8 byte parser and packaged/text sniffing |
| `front_matter` | front matter safe-subset handling through the byte parser |
| `block_parser` | block parser |
| `inline_parser` | inline parser |
| `attrs` | attribute-name validation and attribute parsing through source text |
| `url` | URL and package path policy |
| `package` | stored-ZIP package reader |
| `nods` | safe NODS style audit and sanitizer |
| `ncp` | semantic NCP serializer |
| `navigation` | declarative `toc` navigation resolver |

## Smoke Run

Use the committed smoke script before release hardening changes:

```sh
rtk sh scripts/fuzz_smoke.sh
```

By default this runs every target for 5 000 libFuzzer executions. Override it
when you need a longer local sweep:

```sh
NODX_FUZZ_RUNS=25000 rtk sh scripts/fuzz_smoke.sh
NODX_FUZZ_TARGETS="parse_bytes package" rtk sh scripts/fuzz_smoke.sh
```

## Release Budget

The NODX 1.0 release-candidate budget is the committed smoke script on the
release commit, plus the weekly `Fuzz smoke` GitHub Actions workflow. This is
intentionally bounded so it can run reliably on ordinary project infrastructure.
For high-risk parser changes, maintainers should raise `NODX_FUZZ_RUNS` and
record the target name, git commit, host, duration, run count, and result in
release notes.

```sh
NODX_FUZZ_RUNS=5000 rtk sh scripts/fuzz_smoke.sh
```

Accepted findings must be documented in `SECURITY.md`.

## Current Status

The target entry points exist. Release readiness requires a green local or CI
smoke run on the release commit.

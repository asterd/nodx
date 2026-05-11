# NODX Fuzzing

These targets exercise public parser, validator-adjacent, package, style, URL,
NCP, and navigation boundaries. They are intentionally kept outside the main
Cargo workspace so normal `rtk cargo test` does not require `cargo-fuzz`.

## Setup

Install `cargo-fuzz` in the local Rust toolchain:

```sh
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

Use short local runs before committing harness changes:

```sh
rtk cargo fuzz run parse_bytes -- -runs=1000
rtk cargo fuzz run front_matter -- -runs=1000
rtk cargo fuzz run block_parser -- -runs=1000
rtk cargo fuzz run inline_parser -- -runs=1000
rtk cargo fuzz run attrs -- -runs=1000
rtk cargo fuzz run url -- -runs=1000
rtk cargo fuzz run package -- -runs=1000
rtk cargo fuzz run nods -- -runs=1000
rtk cargo fuzz run ncp -- -runs=1000
rtk cargo fuzz run navigation -- -runs=1000
```

## Release Budget

The NODX 1.0 release-candidate budget is at least 24 CPU-hours per target on
the release branch:

```sh
rtk cargo fuzz run parse_bytes
rtk cargo fuzz run front_matter
rtk cargo fuzz run block_parser
rtk cargo fuzz run inline_parser
rtk cargo fuzz run attrs
rtk cargo fuzz run url
rtk cargo fuzz run package
rtk cargo fuzz run nods
rtk cargo fuzz run ncp
rtk cargo fuzz run navigation
```

Record target name, git commit, host, duration, and result in release notes.
Accepted findings must be documented in `SECURITY.md`.

## Current Status

The target entry points exist. The full 24 CPU-hour per-target release budget
has not been completed in this local wave.

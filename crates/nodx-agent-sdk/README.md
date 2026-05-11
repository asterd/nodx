# nodx-agent-sdk

Validated local mutations for the NODX Agent Mutate 1.1 profile.

This crate applies supervised AST-level mutations to an already parsed
`nodx_core::Document`. Each operation resolves a target by ID, path, or node
hash, requires the caller to provide the target's `beforeHash`, validates the
document after the operation, and emits deterministic `nodx/change/1.1` JSONL
records.

The SDK is intentionally local-only. It does not call LLM APIs, does not require
trusted signatures for local changes, and does not attempt lossless source
rewrites.

# NODX Agent Mutate 1.1

The Agent Mutate profile is implemented by `crates/nodx-agent-sdk`.

The SDK applies local, supervised mutations to a parsed `nodx_core::Document`.
It does not integrate with LLM APIs, does not require trusted signatures for
local changes, and does not attempt lossless source rewrites.

Implemented operations:

- `insert`
- `replace`
- `delete`
- `add-attribute`
- `set-attribute`
- `remove-attribute`
- `add-comment`
- `approve`
- `reject`

Targets resolve by stable ID, numeric AST path, or node hash. Every operation
requires `beforeHash`; the SDK computes `afterHash` for deterministic
`nodx/change/1.1` JSONL records.

Batches are atomic. The SDK applies operations to a cloned document, validates
after each operation with `nodx-validate`, and only commits the clone back to
the caller when every operation succeeds.

# nodx-cst

Byte-preserving CST support for the NODX Editor 1.2 profile.

This crate stores source bytes as editor state, builds byte spans for block CST
nodes, maps those nodes to the canonical `nodx_core::Document`, and exposes
local patch primitives. Emitting an unmodified CST returns the original bytes
exactly, including line endings, delimiters, whitespace, attribute order, and
recoverable invalid syntax.

The CST is not canonical data. It must not be used for hashing, signing, NCP
semantics, or validation decisions. Agent mutation integration validates through
the existing AST-level `nodx-agent-sdk` path and uses CST patches only when a
minimal source rewrite is available.

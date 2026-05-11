# nodx-sign

`nodx-sign` implements the NODX Signature 1.1 verification profile without
changing the frozen NODX 1.0 canonical AST contract.

The signed payload is the ASCII `sha256-BASE64URL_WITHOUT_PADDING` digest of
`nodx_core::canonical_json(document)`. Detached compact JWS uses an empty middle
segment and verifies over the detached digest payload.

This crate does not introduce a second canonicalizer. It treats
`nodx_core::canonical_json` as the frozen NODX 1.0 canonical AST contract:
object keys are emitted in deterministic order, no insignificant whitespace is
emitted, and string escaping is deterministic for the supported AST value set.
That is the signature input; source CST trivia is never inspected.

## Threat Note

ES256 is mandatory and is implemented with RustCrypto `p256`, which is widely
used, pure Rust, and avoids network or platform key fetching. Verification only
accepts compact JWS with `alg: "ES256"` and a 64-byte raw ECDSA signature as
specified by JWS.

EdDSA is intentionally not enabled in this wave. Adding it would introduce a
second signature implementation and key format surface; that should happen only
with explicit dependency review and vectors for Ed25519 keys and tampering.

Cryptographic validity and trust are separate. `TrustPolicy` resolves a key and
returns a trust decision, while signature verification independently reports
whether the JWS cryptographic check succeeded.

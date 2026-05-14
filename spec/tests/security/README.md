# Security corpus

Each `.nodx` fixture under `yaml-hostile/`, `nods-hostile/`, and `xss/` is
hostile by construction. The matching `expectations.json` in the same
directory pins:

- `codes`: the sorted, de-duplicated set of diagnostic codes the
  reference pipeline (parser → validator → style audit → URL policy) must
  emit for the fixture;
- `exit_code`: the exit code `nodx-cli` must return when handed the
  fixture (`0` for benign control inputs, `2` for parse/validation
  failures, `3` for unsupported required profiles).

The contract is checked by
[`crates/nodx-validate/tests/security_corpus_expectations.rs`](../../../crates/nodx-validate/tests/security_corpus_expectations.rs)
on every `cargo test`. A regression that silently accepts a `javascript:`
URL, a forbidden NODS rule, or a YAML alias would flip that test red.

## URL policy and packaging

The URL policy corpus lives in [`url-policy.tsv`](./url-policy.tsv) and
is consumed by `nodx-url`'s unit tests. The packaged-NODX hostile corpus
is generated in-process from `crates/nodx-package` and documented in
[`package-corpus.md`](./package-corpus.md), so no binary fixtures are
committed.

## Adding a fixture

1. Drop the new `.nodx` under the appropriate category.
2. Run the reference CLI once to capture the expected behavior:

   ```sh
   target/release/nodx diagnostics path/to/fixture.nodx --format json
   echo "exit=$?"
   ```

3. Add the entry to the category's `expectations.json` (keys sorted,
   `codes` sorted and de-duplicated).
4. `cargo test -p nodx-validate --test security_corpus_expectations` must
   stay green.

The expectations file is part of the conformance contract: a change in
the emitted codes for a security fixture must be deliberate and
explained in the PR description.

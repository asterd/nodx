# Package Security Corpus

The executable package corpus lives in `crates/nodx-package` unit tests so the
ZIP bytes can be generated deterministically without committing binary archives.

Covered cases:

- manifest digest mismatch emits `NODX-E021`;
- CRC mismatch is rejected;
- duplicate names after package-path normalization are rejected;
- Unix symlink entries are rejected;
- nested ZIP payloads are rejected;
- valid packages are exposed through read-only `PackageFs`.

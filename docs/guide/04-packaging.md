# Packaging a document

NODX has two on-disk shapes:

- **Text NODX** — a `.nodx` file that starts with optional `---` front
  matter and contains UTF-8 text. This is what you edit.
- **Packaged NODX** — a `.nodx` file whose first four bytes are `PK\x03\x04`.
  A ZIP container holds the source plus all its local assets, components,
  and themes. This is what you ship.

The same extension, two formats; the parser detects which by sniffing the
first bytes.

## When you want a package

- The document references images, fonts, or other binary assets.
- The document depends on a custom theme (`.nods`) or component templates.
- You want a single file you can hand off to a recipient with everything
  inside, hashed and verifiable.
- You want exports (PDF/DOCX/PPTX) to bundle the same assets the renderer
  used.

If your document is plain text and links only to public URLs, you do not
need a package.

## Layout

```
my-doc.nodx          ← single ZIP file with this extension
├─ mimetype          ← exactly "application/nodx+zip", stored uncompressed
├─ manifest.yaml     ← entry list with sizes and SHA-256 digests
├─ doc.nodx          ← the document source
├─ assets/
│  ├─ cover.png
│  └─ logo.svg
├─ components/
│  └─ approval-card.nodx
└─ themes/
   └─ brand.nods
```

Rules the package reader enforces:

| Rule | Why |
|---|---|
| First entry is `mimetype` and uncompressed | Sniff stability without inflating |
| Manifest sizes match actual entries | Lying manifests are rejected up front |
| Each entry's SHA-256 matches the manifest | Tamper detection |
| No nested ZIP entries | Closes a known zip-bomb amplification path |
| No path traversal (`..`, absolute paths, special files) | Containment |
| Compression ratio ≤ 100× | Zip-bomb defense |
| Total uncompressed size ≤ 256 MiB | Sanity bound |
| At most 1 024 entries | Sanity bound |

All caps are configurable via `ResourceLimits`. The defaults are tuned for
"big real documents" rather than "the absolute maximum the format can express".

## Building a package

The repository ships a build script:

```sh
python3 scripts/build_package.py
```

It looks for `examples/extended-showcase/` and produces
`examples/extended-showcase-bundled.nodx`. To package your own document,
copy the script, point it at your folder, and adjust the manifest entries.

A minimal `manifest.yaml`:

```yaml
schema: nodx-package/1.0
entry: doc.nodx
entries:
  - path: doc.nodx
    size: 4321
    sha256: sha256-ungWv48Bz-pBQUDeXa4iI7ADYaOWF3qctBD_YfIAFa0
  - path: assets/cover.png
    size: 218742
    sha256: sha256-4f4f...
components:
  - path: components/approval-card.nodx
themes:
  - path: themes/brand.nods
```

The `components:` and `themes:` lists are how the reader knows which entries
are *extensions* to apply during rendering, not generic data.

## Inspecting a package

```sh
nodx package inspect my-doc.nodx
```

Prints the manifest, every entry, its declared size and SHA-256, and any
validation diagnostic. Use it before opening a package from an untrusted
source.

## Verifying a package

```sh
nodx package verify my-doc.nodx
```

Streams every entry through SHA-256 and compares the result against the
manifest. Exit code `0` means every entry matches; `2` means the package
has been altered since it was built.

## Programmatic use

Library APIs for working with packages:

| Language | Open + apply extensions | Render with extensions |
|---|---|---|
| Rust | `nodx_package::Package::open(bytes)` | `nodx_package::apply_package_extensions(&doc, &pkg)` |
| JavaScript | `parsePackagedDocument(bytes)` | `applyPackageExtensions(doc, pkg)` |
| Python | `parse_packaged_document(bytes_)` | `apply_package_extensions(doc, pkg)` |

The reader never reaches the network and never writes the package to disk.
That is intentional: a hosting application can copy the bytes out, hand them
to NODX, and trust that nothing surprising happens between `open` and the
returned in-memory view.

## Signing a package

Package signing is reserved for the Signature profile. The current CLI
`nodx package verify` validates the ZIP structure, manifest, entry sizes,
SHA-256 digests, and document diagnostics; it does not verify a signature.

The experimental `nodx-sign` crate verifies ES256 (ECDSA P-256, SHA-256)
detached JWS signatures over a document's canonical AST. Trust resolution is
pluggable and belongs to the host or release pipeline, not to the document
itself. See [`crates/nodx-sign`](../../crates/nodx-sign) for the current API.

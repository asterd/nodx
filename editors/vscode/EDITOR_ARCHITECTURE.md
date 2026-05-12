# NODX VS Code Editor Architecture

This extension is the reference shape for first-party NODX editor plugins.

## Goals

- Open both plain `.nodx` documents and packaged `.nodx` ZIP files.
- Keep package internals that are not meant for authors hidden from the UI.
- Let authors edit only content, styles, and component definitions.
- Let authors import local files or remote `http/https` URLs as package assets
  and reference them through package-relative paths.
- Save packages by rebuilding `manifest.yaml` with fresh sizes and SHA-256
  digests, then writing a stored ZIP accepted by the reference package reader.
- Render preview and semantic preview through the standard `nodx-js` runtime.
- Show document headers, package integrity, and signature status without
  over-claiming trust.

## Editor Model

The extension uses a binary `CustomEditorProvider` instead of a text document
provider. That choice matters because packaged NODX files are ZIP containers
with a `.nodx` extension; treating them as text corrupts the authoring model.

The provider loads package bytes through `openStoredPackage`, then separates
files into two groups:

- editable: `.nodx`, `.nods`, and `.nodc` entries;
- hidden: assets, keys, signatures, history, and other package entries.

Hidden entries are preserved on save but are not exposed in the editor tree.
Editable entries are shown as content, style, or component files.

## Asset Handling

NODX distinguishes links from assets:

- text links may use safe absolute `http`, `https`, `mailto`, and `tel` URLs;
- `image`, `media`, and `embed` `src` attributes use the asset policy, which
  accepts package-relative paths and safe image data URIs, not remote URLs.

For that reason the editor never writes a remote URL directly into an asset
node. The Add Asset actions either copy a local file or download a remote URL
into `assets/`, choose a collision-free package path, and insert an `image`,
`media`, `embed`, or path-only reference. If the current document is plain
source, adding an asset promotes it to a NODX ZIP package on save.

The preview resolves package-local images to data URIs in the sandboxed
webview. `media` and `embed` remain fallback-rendered because active playback
and remote fetching are outside the NODX 1.0 rendering contract.

## Save Contract

Plain documents save as UTF-8 NODX source.

Packaged documents save as `application/nodx+zip` stored ZIP packages:

1. `mimetype` is written first and uncompressed.
2. Editable files are encoded as UTF-8.
3. Hidden package entries are copied unchanged.
4. `manifest.yaml` is regenerated with current sizes and SHA-256 base64url
   digests.
5. ZIP local headers, central directory, CRC-32, and EOCD are written by the
   extension with no runtime dependency.

When an editable entry changes, the editor does not preserve the manifest
`signature:` pointer because the extension cannot re-sign the package. The old
signature file remains as a hidden historical entry, but it is no longer
advertised as the current document signature.

This keeps package save behavior deterministic and compatible with the current
reference reader, which intentionally accepts only stored ZIP entries.

## Trust Display

The Info action reports package integrity separately from cryptographic trust.
If the reference package reader accepts the archive, manifest digests are
reported as verified. Detached signatures are reported as present but not
cryptographically verified until a trusted key policy exists in the extension.

## UX

The editor follows VS Code's Markdown preview pattern:

- toolbar buttons switch between Preview, Source, Semantic, and Validate;
- Source mode shows an editable package tree without exposing hidden entries;
- command palette actions create plain or packaged documents;
- preview and semantic preview can be opened beside the source editor;
- syntax snippets and completions remain available for normal text editing.

## Current Limits

- Signature verification is deliberately not marked verified without trust
  policy configuration.
- The source editor inside the webview is intentionally simple; VS Code native
  IntelliSense is still available when opening a plain file as text.
- Package assets are preserved and validated, but preview asset URL rewriting is
  not yet implemented.

# NODX Language Support

Syntax highlighting, language configuration, snippets, validation, preview, and
a package-aware editor for NODX 1.0 documents.

## Features

- Open `.nodx` files with the NODX Editor custom editor.
- Use the NODX Editor as the default editor for `.nodx` files.
- Render preview using the bundled `nodx-js` reference runtime.
- Switch to Source mode, including package tree editing for editable content,
  style, and component entries.
- Keep plain source editing full-width, with no package sidebar unless the file
  is a ZIP package.
- Preserve hidden package entries such as assets, keys, history, and signatures.
- Add local files or remote `http/https` URLs as package assets under
  `assets/`, then insert relative `image`, `media`, or `embed` references.
- Save packaged NODX as stored ZIP with regenerated `manifest.yaml` digests.
- Show document headers, package integrity, and signature status from the Info
  action.
- Open Preview or Semantic Preview beside the current editor.
- Validate documents and surface NODX diagnostics.
- Create a plain NODX document or a packaged NODX document from the command
  palette.

Remote URLs are allowed by the NODX runtime for text links only. Asset nodes
such as `image`, `media`, and `embed` must use data image URIs or package-local
paths, so the editor downloads remote assets into the package before inserting
the reference.

Install from a local checkout:

```sh
code --install-extension nodx-language-0.1.0.vsix
```

Build the VSIX package:

```sh
npx @vscode/vsce package
```

See `EDITOR_ARCHITECTURE.md` for the reference editor design and save contract.

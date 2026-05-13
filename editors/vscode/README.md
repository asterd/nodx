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
- Enable remote asset sources for plain documents. The command updates front
  matter with `features.remote-assets: true`.
- Save packaged NODX as stored ZIP with regenerated `manifest.yaml` digests.
- Show document headers, package integrity, and signature status from the Info
  action.
- Open Preview or Semantic Preview beside the current editor.
- Validate documents and surface NODX diagnostics.
- Create a plain NODX document or a packaged NODX document from the command
  palette.

Remote asset URLs are opt-in. Use **Enable Remote Asset Sources** when a plain
document should preserve `http`/`https` image or media URLs in preview. For
portable distribution, prefer adding the remote asset to the package so the
editor downloads it and inserts a relative path.

Install from a local checkout:

```sh
code --install-extension nodx-language-0.1.0.vsix
```

Build the VSIX package:

```sh
npx @vscode/vsce package
```

See `EDITOR_ARCHITECTURE.md` for the reference editor design and save contract.

# NODX Language Support

Syntax highlighting, language configuration, folding markers, and snippets for
NODX 1.0 documents.

Install from a local checkout:

```sh
code --install-extension nodx-language-0.1.0.vsix
```

Build the VSIX package:

```sh
npx @vscode/vsce package
```

This extension is intentionally text-only. Use the `nodx` CLI for validation,
rendering, package inspection, and diagnostics.

# NODX Editor Integration Starters

These files are intentionally small starter integrations for NODX 1.0 syntax
highlighting.

## VSCode

`editors/vscode` is a lightweight extension skeleton with TextMate highlighting,
language configuration, folding markers, and authoring snippets:

```sh
cd editors/vscode
vsce package
```

It contributes the `.nodx` language, a TextMate grammar, bracket/indentation
rules, and snippets for documents, blocks, TOCs, YAML style blocks, figures, and
tables.

## Sublime Text

Copy `editors/sublime/NODX.sublime-syntax` into a Sublime package directory.

## Notepad++

Import `editors/notepad-plus-plus/nodx-udl.xml` through Notepad++ User Defined
Language.

These integrations do not implement validation or preview. Use the CLI for
authoritative parsing and diagnostics.

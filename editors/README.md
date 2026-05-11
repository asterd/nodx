# NODX Editor Integration Starters

These files are intentionally small starter integrations for NODX 1.0 syntax
highlighting.

## VSCode

`editors/vscode` is a minimal extension skeleton:

```sh
cd editors/vscode
vsce package
```

It contributes the `.nodx` language and a TextMate grammar.

## Sublime Text

Copy `editors/sublime/NODX.sublime-syntax` into a Sublime package directory.

## Notepad++

Import `editors/notepad-plus-plus/nodx-udl.xml` through Notepad++ User Defined
Language.

These integrations do not implement validation or preview. Use the CLI for
authoritative parsing and diagnostics.

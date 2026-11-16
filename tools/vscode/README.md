# MosaicBoard MB5 language support

This folder is a source VS Code extension for `.mb5` files. It provides language association, bracket matching, basic highlighting, and snippets. It has no runtime dependencies.

To launch a development host:

```sh
code --extensionDevelopmentPath="$(pwd)/tools/vscode" .
```

To build a `.vsix`, install the optional `vsce` package and run `vsce package` from this folder. The extension is intentionally small and does not claim full semantic validation; use `tools/mb5/check.sh` for the project and script checks.

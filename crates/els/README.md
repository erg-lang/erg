# els (erg-language-server)

ELS is a language server for the [Erg](https://github.com/erg-lang/erg) programming language.

## Features

- [x] Syntax highlighting (by [vscode-erg](https://github.com/erg-lang/vscode-erg))
- [x] Code completion
  - [x] Variable completion
  - [x] Method/attribute completion
  - [x] Smart completion (considering type, parameter names, etc.)
  - [x] Auto-import
- [x] Diagnostics
- [x] Hover
- [x] Go to definition
- [x] Go to type definition
- [x] Go to implementation
- [x] Find references
- [x] Renaming
- [x] Inlay hint
- [x] Semantic tokens
- [x] Code actions
  - [x] eliminate unused variables
  - [x] change variable case
  - [x] extract variables/functions
  - [x] inline variables
- [x] Code lens
  - [x] show trait implementations
- [x] Signature help
- [x] Workspace symbol
- [x] Document symbol
- [x] Document highlight
- [x] Document link
- [x] Call hierarchy
- [x] Folding range
  - [x] Folding imports
- [x] Selection range

## Installation

```console
cargo install erg --features els
```

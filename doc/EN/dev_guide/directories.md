# Directory Structure of Erg

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/dev_guide/directories.md%26commit_hash%3Dfc7a25a8d86c208fb07beb70ccc19e4722c759d3)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/dev_guide/directories.md&commit_hash=fc7a25a8d86c208fb07beb70ccc19e4722c759d3)

```console
 └─┬ assets: images
   ├─ CODE_OF_CONDUCT: Code of Conduct
   ├─┬ compiler
   │ ├─ erg_common: common utilities
   │ ├─ erg_compiler: Compiler
   │ └─ erg_parser: Parser
   ├─┬ doc
   │ ├─┬ EN
   │ │ ├─ API: Erg standard API
   │ │ ├─ compiler: about implementation of the compiler
   │ │ ├─ dev_guide: guide for developers & contributors
   │ │ ├─ python: Knowledge of Python required for Erg development
   │ │ ├─ syntax: syntax of Erg
   │ │ └─ tools: about Erg's CLI tools
   │ └─┬ JA
   │  ...
   ├─ examples: sample code
   ├─ library: Erg libraries
   ├─ src: main.rs & driver
   └─ tests: test code
```

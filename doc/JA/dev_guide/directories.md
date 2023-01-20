# Ergリポジトリのディレクトリ構造

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/dev_guide/directories.md%26commit_hash%3D94185d534afe909d112381b53d60895389d02f95)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/dev_guide/directories.md&commit_hash=94185d534afe909d112381b53d60895389d02f95)

```console
 └─┬ assets: 画像など
   ├─ CODE_OF_CONDUCT: 行動規範
   ├─┬ crates
   │ ├─ els: Erg Language Server (言語サーバー)
   │ ├─ erg_common: 共通のユーティリティ
   │ ├─ erg_compiler: コンパイラ、**Ergのコア**
   │ └─ erg_parser: パーサー
   ├─┬ doc
   │ ├─┬ EN
   │ │ ├─ API: Erg標準API
   │ │ ├─ compiler: コンパイラの実装に関して
   │ │ ├─ dev_guide: 開発・貢献者向けガイド
   │ │ ├─ python: Ergの開発に必要なPythonの知識
   │ │ ├─ syntax: Ergの文法
   │ │ └─ tools: Ergのコマンドラインツールに関して
   │ └─┬ JA
   │  ...
   ├─ examples: サンプルコード
   ├─ library: Ergスクリプトによるライブラリ
   ├─ src: main.rsとドライバの置かれたディレクトリ
   └─ tests: テストコード
```

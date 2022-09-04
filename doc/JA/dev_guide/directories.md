# Ergリポジトリのディレクトリ構造

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/dev_guide/directories.md%26commit_hash%3Deccd113c1512076c367fb87ea73406f91ff83ba7)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/dev_guide/directories.md&commit_hash=eccd113c1512076c367fb87ea73406f91ff83ba7)

```console
 └─┬ assets: 画像など
   ├─ CODE_OF_CONDUCT: 行動規範
   ├─┬ compiler
   │ ├─ erg_common: 共通のユーティリティ
   │ ├─ erg_compiler
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

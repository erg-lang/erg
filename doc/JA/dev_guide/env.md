# 開発環境

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/dev_guide/env.md%26commit_hash%3D13f2d31aee9012f60b7a40d4b764921f1419cdfe)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/dev_guide/env.md&commit_hash=13f2d31aee9012f60b7a40d4b764921f1419cdfe)

## インストールが必要なもの

* Rust (installed with rustup)

  * ver >= 1.64.0
  * 2021 edition

* [pre-commit](https://pre-commit.com/)

pre-commitを使ってclippyのチェックやテストを自動で行わせています。
バグがなくても最初の実行でチェックが失敗する場合があります。その場合はもう一度コミットを試みてください。

* Python3インタープリタ

## 推奨

* エディタ: Visual Studio Code
* VSCode拡張機能: Rust-analyzer, GitLens, Git Graph, GitHub Pull Requests and Issues, Markdown All in One, markdownlint
* OS: Windows 10/11 | Ubuntu 20.04/22.04 | macOS Monterey/Ventura

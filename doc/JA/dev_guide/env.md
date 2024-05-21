# 開発環境

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/dev_guide/env.md%26commit_hash%3Dcbaf48c04b46fadc680fa4e05e8ad22cbdaf6c47)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/dev_guide/env.md&commit_hash=cbaf48c04b46fadc680fa4e05e8ad22cbdaf6c47)

## インストールが必要なもの

* Rust (installed with rustup)

  * ver >= 1.64.0
  * 2021 edition

* [pre-commit](https://pre-commit.com/)

pre-commitを使ってclippyのチェックやテストを自動で行わせています。
バグがなくても最初の実行でチェックが失敗する場合があります。その場合はもう一度コミットを試みてください。

* Python3インタープリタ (3.7~3.11)

様々なバージョンでErgの挙動を検査したい場合は [pyenv](https://github.com/pyenv/pyenv) 等の導入をお勧めします。

## 推奨

* エディタ: Visual Studio Code
* VSCode拡張機能: Rust-analyzer, GitLens, Git Graph, GitHub Pull Requests and Issues, Markdown All in One, markdownlint
* OS: Windows 10/11 | Ubuntu 20.04/22.04 | macOS Monterey/Ventura

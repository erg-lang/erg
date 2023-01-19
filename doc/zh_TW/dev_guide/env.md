# 開發環境

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/dev_guide/env.md%26commit_hash%3D14b0c449efc9e9da3e10a09c912a960ecfaf1c9d)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/dev_guide/env.md&commit_hash=14b0c449efc9e9da3e10a09c912a960ecfaf1c9d)

## 你需要安裝什麽

* Rust(與 rustup 一起安裝)

    * 版本 >= 1.64.0
    * 2021年版

* [pre-commit](https://pre-commit.com/)

我們使用pre-commit來自動進行clippy檢查和測試。即使沒有錯誤，檢查也可能在第一次運行時失敗，在這種情況下，您應該再次嘗試提交。

* Python3 解釋器

## 推薦

* 編輯器: Visual Studio Code
* VSCode 擴展: Rust-analyzer、GitLens、Git Graph、GitHub Pull Requests and Issues、Markdown All in One、markdownlint
* 操作系統: Windows 10/11 | Ubuntu 20.04/22.04 | macOS Monterey/Ventura

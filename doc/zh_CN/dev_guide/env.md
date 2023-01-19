# 开发环境

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/dev_guide/env.md%26commit_hash%3D14b0c449efc9e9da3e10a09c912a960ecfaf1c9d)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/dev_guide/env.md&commit_hash=14b0c449efc9e9da3e10a09c912a960ecfaf1c9d)

## 你需要安装什么

* Rust(与 rustup 一起安装)

    * 版本 >= 1.64.0
    * 2021年版

* [pre-commit](https://pre-commit.com/)

我们使用pre-commit来自动进行clippy检查和测试。即使没有错误，检查也可能在第一次运行时失败，在这种情况下，您应该再次尝试提交。

* Python3 解释器

## 推荐

* 编辑器: Visual Studio Code
* VSCode 扩展: Rust-analyzer、GitLens、Git Graph、GitHub Pull Requests and Issues、Markdown All in One、markdownlint
* 操作系统: Windows 10/11 | Ubuntu 20.04/22.04 | macOS Monterey/Ventura

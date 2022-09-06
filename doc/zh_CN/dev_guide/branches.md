# 分支机构命名和运营策略

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/dev_guide/branches.md%26commit_hash%3Da711efa99b325ba1012f6897e7b0e2bdb947d8a1)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/dev_guide/branches.md&commit_hash=a711efa99b325ba1012f6897e7b0e2bdb947d8a1)

## main

* 主要开发分支
* 必须满足以下条件

* 编译成功

## beta(目前不创建)

* 最新的 Beta 版本
* 必须满足以下条件

* 编译成功
* 所有测试都会成功

## feature-*

* 开发特定功能的分支
* 切开 main

* 没有条件

## issue-*

* 解决特定 issue 的分支

* 没有条件

## fix-*

* 修复特定错误的分支(如果该问题是一个错误，则代替`issue-*`创建)。

* 没有条件。
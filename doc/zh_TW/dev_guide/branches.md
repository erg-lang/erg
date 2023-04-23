# 分支機構命名和運營策略

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/dev_guide/branches.md%26commit_hash%3Dbf5df01d09e42ec8433a628420e096ac55e4d3e4)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/dev_guide/branches.md&commit_hash=bf5df01d09e42ec8433a628420e096ac55e4d3e4)

## main(基於主幹的開發)
* 主要開發分支
* 必須滿足以下條件

* 編譯成功

## beta(目前不創建)

* 最新的 Beta 版本
* 必須滿足以下條件

* 編譯成功
* 所有測試都會成功

## feature-*(名字)

* 開發特定功能的分支
* 切開 main

* 沒有條件

## issue-*(#issue)

* 解決特定 issue 的分支

* 沒有條件

## fix-*(#issue or bug 名字)

* 修復特定錯誤的分支(如果該問題是一個錯誤，則代替`issue-*`創建)

* 沒有條件。

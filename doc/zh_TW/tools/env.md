# 環境子命令

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/tools/env.md%26commit_hash%3Dd15cbbf7b33df0f78a575cff9679d84c36ea3ab1)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/tools/env.md&commit_hash=d15cbbf7b33df0f78a575cff9679d84c36ea3ab1)

env 子命令指定 erg 執行環境
使用 `erg env new [env name]` 創建一個新的執行環境。 將打開一個交互式工具，當您指定 erg 版本時，將安裝該版本的 erg(如果已存在，將使用它)，您將能夠將其用作新環境
您可以使用 `erg env switch [env name]` 切換環境
可以使用 `erg env edit` 編輯創建的環境以預安裝軟件包并指定其他語言的依賴項
該命令最大的特點是`erg env export`可以將重現環境的信息輸出為`[env name].env.er`文件。 這使您可以立即開始在與其他人相同的環境中進行開發。 此外，`erg env publish` 可以像包一樣發布環境。
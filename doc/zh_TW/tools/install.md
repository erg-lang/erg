# 安裝子命令

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/tools/install.md%26commit_hash%3Dd15cbbf7b33df0f78a575cff9679d84c36ea3ab1)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/tools/install.md&commit_hash=d15cbbf7b33df0f78a575cff9679d84c36ea3ab1)

您可以使用 install 安裝在注冊表站點上注冊的軟件包
基本用法與cargo等包管理器相同

## 便利功能

* 如果有同名的包名，且下載次數超過該包名的10倍以上，會提示可能輸入錯誤。這可以防止拼寫錯誤
* 如果包很大(超過 50MB)，請顯示大小并建議您是否真的要安裝它
* 如果包裝重復，建議使用替代包裝。
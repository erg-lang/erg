# 構建子命令

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/tools/build.md%26commit_hash%3Dd15cbbf7b33df0f78a575cff9679d84c36ea3ab1)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/tools/build.md&commit_hash=d15cbbf7b33df0f78a575cff9679d84c36ea3ab1)

build 子命令構建包
默認構建中執行的步驟如下: 

1. 檢查注釋/文檔中的代碼(doc 下的 md 文件)
2. 編譯打包所需的代碼
3. 對于應用程序包，生成批處理文件或相當于命令的shell腳本
4. 運行測試

構建完成后的交付物輸出到以下目錄

* 在調試構建期間: build/debug
* 對于發布構建: build/release
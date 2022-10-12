# 故障診斷

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/dev_guide/troubleshooting.md%26commit_hash%3Db57b46405734013fee2925f43d4a46ad8898267d)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/dev_guide/troubleshooting.md&commit_hash=b57b46405734013fee2925f43d4a46ad8898267d)

## Q: 本地生成成功, 但 GitHub Actions 生成失敗

A: 您正在處理的分支可能沒有Pull`main`中的更改

## Q: 提交前檢查失敗

A: 嘗試再次提交, 第一次可能會誤判, 如果一次又一次的失敗, 那麼你的代碼可能包含錯誤

## Q: build.rs 無法正常運行

A: 檢查`build. rs`運行目錄中的額外檔案/資料夾（例如`__pychache__`）

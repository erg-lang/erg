# 故障诊断

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/dev_guide/troubleshooting.md%26commit_hash%3D5a347e87e8a72b59ed3f503ade8cde63276c718e)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/dev_guide/troubleshooting.md&commit_hash=5a347e87e8a72b59ed3f503ade8cde63276c718e)

## Q: 本地生成成功, 但 GitHub Actions 生成失败

A: 您正在处理的分支可能没有Pull`main`中的更改

## Q: 提交前检查失败

A: 尝试再次提交, 第一次可能会误判, 如果一次又一次的失败, 那么你的代码可能包含错误

## Q: pre-commit test gives "link failure"

A: Make sure cargo is not running in another process.

## Q: build.rs 无法正常运行

A: 检查 `build.rs` 运行目录中的额外文件/文件夹 (例如 `__pychache__`)

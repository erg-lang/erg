# トラブルシューティング

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/dev_guide/troubleshooting.md%26commit_hash%3De033e4942e70657008427f05def2d1b1bfb5ed66)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/dev_guide/troubleshooting.md&commit_hash=e033e4942e70657008427f05def2d1b1bfb5ed66)


## Q: ローカルでのビルドは成功したが、GitHub Actionsのビルドが失敗する

A: あなたの作業しているブランチが`main`の変更に追従していない可能性があります。

## Q: pre-commitのチェックが失敗する

A: もう一度コミットを試みてください。最初の1回は失敗することがあります。何度やっても失敗する場合、コードにバグが含まれている可能性があります。

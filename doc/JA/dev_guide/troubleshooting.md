# トラブルシューティング

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/dev_guide/troubleshooting.md%26commit_hash%3Db57b46405734013fee2925f43d4a46ad8898267d)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/dev_guide/troubleshooting.md&commit_hash=b57b46405734013fee2925f43d4a46ad8898267d)

## Q: ローカルでのビルドは成功したが、GitHub Actionsのビルドが失敗する

A: あなたの作業しているブランチが`main`の変更に追従していない可能性があります。

## Q: pre-commitのチェックが失敗する

A: もう一度コミットを試みてください。最初の1回は失敗することがあります。何度やっても失敗する場合、コードにバグが含まれている可能性があります。

## Q: pre-commitのテストで「リンク失敗」となる

A: 別のプロセスでcargoが実行中でないか確認してください。

## Q: build.rsの実行に失敗する

A: build.rsが動作するディレクトリ上に余計なファイル・ディレクトリ(`__pychache__`など)がないか確認してください。

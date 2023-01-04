# ブランチの命名と運用方針

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/dev_guide/branches.md%26commit_hash%3Dbf5df01d09e42ec8433a628420e096ac55e4d3e4)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/dev_guide/branches.md&commit_hash=bf5df01d09e42ec8433a628420e096ac55e4d3e4)

* 基本的に開発は`main`ブランチ一本で行う(トランクベース開発)。どうしてもブランチを切らないと作業しにくい場合のみ`feature-*`ブランチか`issue-*`ブランチを作成する。

## main

* メイン開発ブランチ
* 以下の条件を満たす必要がある

* コンパイルが成功する

## beta (現在のところは作らない)

* 最新のベータリリース
* 以下の条件を満たす必要がある

* コンパイルが成功する
* 全てのテストが成功する

## feature-*

* 特定の一機能を開発するブランチ
* mainを切って作る

* 条件なし

## issue-*

* 特定のissueを解決するブランチ

* 条件なし

## fix-*

* 特定のバグを解決するブランチ(issueがバグの場合に、`issue-*`の代わりに作成する)

* 条件なし

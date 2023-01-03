# ブランチの命名と運用方針

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/dev_guide/branches.md%26commit_hash%3D14b0c449efc9e9da3e10a09c912a960ecfaf1c9d)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/dev_guide/branches.md&commit_hash=14b0c449efc9e9da3e10a09c912a960ecfaf1c9d)

* 基本的に開発は`main`ブランチ一本で行う。どうしてもブランチを切らないと作業しにくい場合のみ`feature-*`ブランチか`issue-*`ブランチを作成する。

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

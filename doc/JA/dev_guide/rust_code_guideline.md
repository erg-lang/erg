# Rustコードに関するガイドライン

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/dev_guide/rust_code_guideline.md%26commit_hash%3Deccd113c1512076c367fb87ea73406f91ff83ba7)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/dev_guide/rust_code_guideline.md&commit_hash=eccd113c1512076c367fb87ea73406f91ff83ba7)

## ローカルルール

* デバッグ用の出力には`log!`を使用する(release時にも必要な出力処理は`println!`等を使用する)。
* 未使用・または内部用の(privateかつ特定の機能のみに使用する)変数・メソッドは先頭に`_`を1つ付ける。予約語との衝突を回避したい場合は後ろに`_`を1つ付ける。

## 奨励されるコード

* 数値の列挙やboolの代わりにドメイン固有のEnumを定義して使う。
* アクセス修飾子は必要最小限のものとする。公開する場合でも`pub(mod)`や`pub(crate)`を優先的に使用する。
* for式でのiterableオブジェクトは明示的にイテレータに変換する(`for i in x`ではなく`for i in x.iter()`)。
* 遅延評価。例えば、`default`がリテラル以外の場合は`unwrap_or`ではなく`unwrap_or_else`を使用する。

## 奨励されないコード

* return type overloadingを多用する。具体的には自明でない`.into`を多用するコード。これは型推論結果が直感に反する場合があるためである。この場合は代わりに`from`を使うことを推奨する。
* `Deref`を多用する。これは実質的に継承と同じ問題を引き起こす。

## 文脈により判断が変わるコード

* 未使用のヘルパーメソッドを定義する。
* `unwrap`, `clone`を多用する。場合によってはそうするより他にないものもある。

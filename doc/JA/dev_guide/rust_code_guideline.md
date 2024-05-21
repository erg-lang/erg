# Rustコードに関するガイドライン

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/dev_guide/rust_code_guideline.md%26commit_hash%3D1767df5de23976314a54c3c57bb80be3cb0ddc4f)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/dev_guide/rust_code_guideline.md&commit_hash=1767df5de23976314a54c3c57bb80be3cb0ddc4f)

## ローカルルール

* デバッグ用の出力には`log!`を使用する(release時にも必要な出力処理は`println!`や`BufWriter`等を使用する)。
* 未使用・または内部用の(privateかつ特定の機能のみに使用する)変数・メソッドは先頭に`_`を1つ付ける。予約語との衝突を回避したい場合は後ろに`_`を1つ付ける。
* clippyを使用する。ただしclippyのルールの中にはあまり意味のないものもあるので、レベルがdenyでない場合は`#[allow(clippy::...)]`を使用して無視しても良い。

## 奨励されるコード

* 数値の列挙やboolの代わりにドメイン固有のEnumを定義して使う。
* アクセス修飾子は必要最小限のものとする。公開する場合でも`pub(mod)`や`pub(crate)`を優先的に使用する。
* for式でのiterableオブジェクトは明示的にイテレータに変換する(`for i in x`ではなく`for i in x.iter()`)。
* 遅延評価。例えば、`default`がリテラル以外の場合は`unwrap_or`ではなく`unwrap_or_else`を使用する。
* `debug_assert!`, `debug_assert_eq!`, `debug_power_assert!`等のアサーションを活用する。`debug_assert!(..., "{x} is not ...");`のようにエラーメッセージも指定する。

## 奨励されないコード

* return type overloadingを多用する。具体的には自明でない`.into`を多用するコード。これは型推論結果が直感に反する場合があるためである。この場合は代わりに`from`を使うことを推奨する。
* `Deref`を多用する。これは実質的に継承と同じ問題を引き起こす。

## 文脈により判断が変わるコード

* 未使用のヘルパーメソッドを定義する。
* `unwrap`, `clone`を多用する。場合によってはそうするより他にないものもある。

## 依存関係

極力依存関係は少なくし、必要なものは自前で実装する。実装が極めて困難、またはハードウェア依存性が高いなどの場合のみ外部依存を許す(例: `libc`, `winapi`)。
また、外部依存がないcrateは使用して良い(例: `unicode-xid`)。そうでない場合はoptional dependencyとして認める場合がある。いずれの場合も、良くメンテナンスされており、使用例の多いものを選択する。

また、このルールが適用されるのはErgコンパイラ本体のみであり、Ergのツールやライブラリは自由に依存関係を追加して良い。

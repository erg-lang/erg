# Name resolving (名前解決)

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/compiler/phases/04_name_resolve.md%26commit_hash%3D19bab4ae63af9415da20ebd7499c668144da5ea6)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/compiler/phases/04_name_resolve.md&commit_hash=19bab4ae63af9415da20ebd7499c668144da5ea6)

Ergの名前解決フェーズは現在のところ型解析フェーズと一体化している。
これは良い設計であるとはいえず、将来的には分離される予定である。

名前解決フェーズで行われることは、以下の通りである。

* 変数名をスコープに対応付け、ユニークなIDを割り当て、必要ならば型変数を割り当てる
* 定数を依存関係に従って並び替える
* 定数式を評価し、可能ならば置換する(これは名前解決フェーズから分離される可能性がある)

# Name resolving　(名前解決)

Ergの名前解決フェーズは現在のところ型解析フェーズと一体化している。
これは良い設計であるとはいえず、将来的には分離される予定である。

名前解決フェーズで行われることは、以下の通りである。

* 変数名をスコープに対応付け、ユニークなIDを割り当て、必要ならば型変数を割り当てる
* 定数を依存関係に従って並び替える
* 定数式を評価し、可能ならば置換する(これは名前解決フェーズから分離される可能性がある)

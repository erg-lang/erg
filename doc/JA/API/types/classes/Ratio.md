# Ratio

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/types/classes/Ratio.md%26commit_hash%3Dd15cbbf7b33df0f78a575cff9679d84c36ea3ab1)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/types/classes/Ratio.md&commit_hash=d15cbbf7b33df0f78a575cff9679d84c36ea3ab1)

有理数を表す型です。主に、分数を使いたいときに使います。
実はErgでの/演算子はRatioを返します。1/3などは0.33333...とは評価されず1/3のまま処理されます。また、0.1は1/10と等価です。なので、`0.1 + 0.2 == 0.3`となります。当たり前のように聞こえますが、PythonではなんとFalseになります。
とはいえ、Ratio型はFloat型に比べて若干効率が下がる傾向があります。そこまで正確な数値は要らず、かつ実行速度が重要となるポイントではFloat型を使用するといいでしょう。とはいえ、Rob Pikeが言うように早すぎる最適化は諸悪の根源です。Ratio型を捨ててFloat型を使用するのは実際にパフォーマンステストを行ってからにしましょう。素人ほど無条件に軽量な型を好みます。

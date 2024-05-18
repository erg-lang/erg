# Side-effect checking (副作用検査)

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/compiler/phases/06_effect_check.md%26commit_hash%3D19bab4ae63af9415da20ebd7499c668144da5ea6)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/compiler/phases/06_effect_check.md&commit_hash=19bab4ae63af9415da20ebd7499c668144da5ea6)

副作用検査はEffectCheckerという構造体で実行される。
副作用検査では以下のことを行う。

* 副作用の許されない式中で副作用のある呼び出しが行われていないか検査する。
* 関数中で可変オブジェクトが参照されていないか検査する
* 関数にプロシージャが代入されていないか検査する

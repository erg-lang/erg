# Desugaring (脱糖)

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/compiler/phases/03_desugar.md%26commit_hash%3D19bab4ae63af9415da20ebd7499c668144da5ea6)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/compiler/phases/03_desugar.md&commit_hash=19bab4ae63af9415da20ebd7499c668144da5ea6)

Ergは型解析以降の処理が肥大化することを避けるため、構文解析の段階でいくつかの構文糖を脱糖する。
代表的な糖衣構文はパターンである。全てのパターンは単なる変数代入と型指定の組み合わせに還元される。

# Tuple *T: Type

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/types/classes/Tuple.md%26commit_hash%3D8673a0ce564fd282d0ca586642fa7f002e8a3c50)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/types/classes/Tuple.md&commit_hash=8673a0ce564fd282d0ca586642fa7f002e8a3c50)

複数の型のオブジェクトを保持するコレクション。

## methods

* zip self, other

    2つの順番付けられたコレクション(配列かタプル)を合成する。

    ```python
    assert ([1, 2, 3].zip [4, 5, 6])[0] == (1, 4)
    ```

* zip_by self, op, other

    zipを一般化したメソッド。合成するための二項演算を指定できる。
    演算子には`()`, `[]`, `{}`, `{:}`も指定可能で、それぞれタプル, 配列, セット, ディクトを生成する。

    ```python
    assert ([1, 2, 3].zip([4, 5, 6]))[0] == (1, 4)
    assert ([1, 2, 3].zip_by((),  [4, 5, 6]))[0] == (1, 4)
    assert ([1, 2, 3].zip_by([],  [4, 5, 6]))[0] == [1, 4]
    assert ([1, 2, 3].zip_by({},  [4, 5, 6]))[0] == {1, 4}
    assert ([1, 2, 3].zip_by({:},  [4, 5, 6]))[0] == {1: 4}
    assert ([1, 2, 3].zip_by(`_+_`, [4, 5, 6]))[0] == 5
    ```

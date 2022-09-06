# Str

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/types/classes/Str.md%26commit_hash%3Dd15cbbf7b33df0f78a575cff9679d84c36ea3ab1)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/types/classes/Str.md&commit_hash=d15cbbf7b33df0f78a575cff9679d84c36ea3ab1)

(不変長)文字列を表す型。単なる`Str`型は`StrWithLen N`型から文字数の情報を落とした型である(`Str = StrWithLen _`)。

## methods

* isnumeric

文字列がアラビア数字であるかを返す。漢数字やその他の数字を表す文字の判定は`isunicodenumeric`を使う(この挙動はPythonと違うので注意)。

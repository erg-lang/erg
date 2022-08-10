# Str

(不変長)文字列を表す型。単なる`Str`型は`StrWithLen N`型から文字数の情報を落とした型である(`Str = StrWithLen _`)。

## methods

* isnumeric

文字列がアラビア数字であるかを返す。漢数字やその他の数字を表す文字の判定は`isunicodenumeric`を使う(この挙動はPythonと違うので注意)。

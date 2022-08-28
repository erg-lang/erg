# REPL

`erg`コマンドを引数を与えず実行すると、REPLが起動されます。また、`repl`サブコマンドを指定して起動することもできます。
さらに以下のフラグを指定できます。

* typed: オブジェクトとその型を表示します。

```console
$ erg repl --typed
Erg interpreter ... (tags/?:, ...) on ...
>>> 1
1: {1}
>>> id x = x
id = <function id>: |T: Type| T -> T
```

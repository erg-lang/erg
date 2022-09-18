# REPL

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/tools/repl.md%26commit_hash%3D8dcbcb4235ba73cd2618fe5407a1ea18f7784da1)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/tools/repl.md&commit_hash=8dcbcb4235ba73cd2618fe5407a1ea18f7784da1)

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

# REPL

運行不帶參數的 `erg` 命令會調用 REPL。 它也可以用 `repl` 子命令調用。
此外，您可以指定以下標志：

* typed：顯示對象及其類型。

```console
$ erg repl --typed
Erg interpreter ... (tags/?:, ...) on ...
>>> 1
1: {1}
>>> id x = x
id = <function id>: |T: Type| T -> T
```
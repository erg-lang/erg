# REPL

如果命令沒有參數，則會調用 REPL。你也可以使用<gtr=“3”/>子命令啟動它。還可以指定以下標誌。

* typed：顯示對象及其類型。


```console
$ erg repl --typed
Erg interpreter ... (tags/?:, ...) on ...
>>> 1
1: {1}
>>> id x = x
id = <function id>: |T: Type| T -> T
```
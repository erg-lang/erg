# REPL

运行不带参数的 `erg` 命令会调用 REPL。 它也可以用 `repl` 子命令调用。
此外，您可以指定以下标志：

* typed：显示对象及其类型。

```console
$ erg repl --typed
Erg interpreter ... (tags/?:, ...) on ...
>>> 1
1: {1}
>>> id x = x
id = <function id>: |T: Type| T -> T
```
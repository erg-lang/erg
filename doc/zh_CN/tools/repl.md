# REPL

如果命令没有参数，则会调用 REPL。你也可以使用<gtr=“3”/>子命令启动它。还可以指定以下标志。

* typed：显示对象及其类型。


```console
$ erg repl --typed
Erg interpreter ... (tags/?:, ...) on ...
>>> 1
1: {1}
>>> id x = x
id = <function id>: |T: Type| T -> T
```

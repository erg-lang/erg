# REPL

Running the `erg` command with no arguments invokes the REPL. It can also be invoked with the `repl` subcommand.
Additionally, you can specify the following flags:

* typed: Show objects and their types.

```console
$ erg repl --typed
Erg interpreter ... (tags/?:, ...) on ...
>>> 1
1: {1}
>>> id x = x
id = <function id>: |T: Type| T -> T
```
# REPL

Running the `erg` command with no arguments invokes the REPL. It can also be invoked with the `repl` subcommand.
Additionally, you can specify the following flags:

* show-type: Show objects and their types.

```console
$ erg repl --show-type
Erg interpreter ... (tags/?:, ...) on ...
>>> 1
1: {1}
>>> id x = x
id = <function id>: |T: Type| T -> T
```

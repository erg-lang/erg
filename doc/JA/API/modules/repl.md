# module `repl`

provides REPL(Read-Eval-Print-Loop)-related APIs.

## functions

* `gui_help`

オブジェクトに関する情報をブラウザで表示する。オフラインでも使用可能。

## types

### Guess = Object

#### methods

* `.guess`

与えられた引数と戻り値から、関数を推測する。

```python
1.guess((1,), 2) # <Int.__add__ method>
[1, 2].guess((3, 4), [1, 2, 3, 4]) # <Array(T, N).concat method>
```

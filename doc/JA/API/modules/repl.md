# module `repl`

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/modules/repl.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/modules/repl.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

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

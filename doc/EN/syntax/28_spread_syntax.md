# Spread assignment

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/28_spread_syntax.md%26commit_hash%3D6dc8c5015b6120497a26d80eaef65d23eb2bee2a)
](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/28_spread_syntax.md&commit_hash=6dc8c5015b6120497a26d80eaef65d23eb2bee2a)

In a spread assignment, a variable can be prefixed with `...` in front of the variable, all the remaining elements can be expanded into the variable. This is called a spread assignment.

```erg
[x, ... .y] = [1, 2, 3]
assert x == 1
assert y == [2, 3].
x, ... .y = (1, 2, 3)
assert x == 1
assert y == (2, 3)
```

## Extract assignment

If nothing is written after `...`, the remaining elements are ignored and an assignment is made. This type of expansion assignment is specifically called an extract assignment.
Extract assignment is a useful syntax for bringing certain attributes local to a module or record.

```erg
{sin; cos; tan; ...} = import "math"
```

This way, `sin`, `cos`, `tan` can be used locally from then on.

You can do the same with records.

```erg
record = {x = 1; y = 2}
{x; y; ...} = record
```

If you want to expand all of them, use `{*} = record`, this is equivalent to `open` in OCaml and so on.

```erg
record = {x = 1; y = 2}
{*} = record
assert x == 1 and y == 2
```

<p align='center'>
    <a href='./27_comprehension.md'>Previous</a> | <a href='./29_decorator.md'>Next</a>
</p>

# operator

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/06_operator.md%26commit_hash%3D21e8145e83fb54ed77e7631deeee8a7e39b028a3)
](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/06_operator.md&commit_hash=21e8145e83fb54ed77e7631deeee8a7e39b028a3)

Operators are symbols that represent operations. Operands are things to the (left) right of an operator.

Operators are a kind of function, and thus are themselves first-class objects that can be bound to variables. When binding, it is necessary to enclose it with ``.
For `+` (and `-`), there are both unary and binary operators, so `_+_`(binary operation)/`+_`(unary operation ) must be specified.

``` erg
add = `+` # SyntaxError: specify `_+_` or `+_`
add=`_+_`
assert f(1, 2) == 3
assert f("a", "b") == "ab"

g = `*` # OK, this is binary only
assert g(1, 2) == 2
```

Some fundamental operators, called special forms, cannot be bound.

``` erg
def = `=` # SyntaxError: cannot bind `=` operator, this is a special form
# NG: def x, 1
function = `->` # SyntaxError: cannot bind `->` operator, this is a special form
# NG: function x, x + 1
```

<p align='center'>
    <a href='./05_builtin_funcs.md'>Previous</a> | <a href='./07_side_effect.md'>Next</a>
</p>

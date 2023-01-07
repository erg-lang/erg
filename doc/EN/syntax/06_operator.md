# operator

Operators are symbols that represent operations. Operands are things to the (left) right of an operator.

Operators are a kind of function, and thus are themselves first-class objects that can be bound to variables. When binding, it is necessary to enclose it with ``.
For `+` (and `-`), there are both unary and binary operators, so `_+_`(binary operation)/`+_`(unary operation ) must be specified.

```python,compile_fail
add = `+` # SyntaxError: specify `_+_` or `+_`
```

```python
add = `_+_`
assert add(1, 2) == 3
assert add("a", "b") == "ab"

mul = `*` # OK, this is binary only
assert mul(1, 2) == 2
```

Some fundamental operators, called special forms, cannot be bound.

```python,compile_fail
def = `=` # SyntaxError: cannot bind `=` operator, this is a special form
# NG: def x, 1
function = `->` # SyntaxError: cannot bind `->` operator, this is a special form
# NG: function x, x + 1
```

<p align='center'>
    <a href='./05_builtin_funcs.md'>Previous</a> | <a href='./07_side_effect.md'>Next</a>
</p>

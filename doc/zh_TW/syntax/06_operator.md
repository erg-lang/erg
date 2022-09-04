# 運算符

運算符（操作符）是表示運算的符號。運算符（操作數）位於運算符的右側（左），在 Erg 中它只是一個對象。

運算符是一種函數，因此它本身也可以綁定到一級對像中的變量。綁定必須用包圍。對於<gtr=“4”/>（和<gtr=“5”/>），必須指定<gtr=“6”/>（二元運算）/<gtr=“7”/>（一元運算）以實現唯一化，因為同時存在一元運算符和二元運算符。


```erg
add = `+` # SyntaxError: specify `_+_` or `+_`
add = `_+_`
assert f(1, 2) == 3
assert f("a", "b") == "ab"

g = `*` # OK, this is binary only
assert g(1, 2) == 2
```

但是，請注意，某些稱為特殊格式的運算符不能被綁定。


```erg
def = `=` # SyntaxError: cannot bind `=` operator, this is a special form
# NG: def x, 1
function = `->` # SyntaxError: cannot bind `->` operator, this is a special form
# NG: function x, x + 1
```

<p align='center'>
    <a href='./05_builtin_funcs.md'>Previous</a> | <a href='./07_side_effect.md'>Next</a>
</p>
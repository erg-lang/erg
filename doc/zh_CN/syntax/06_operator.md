# 运算符

运算符（操作符）是表示运算的符号。运算符（操作数）位于运算符的右侧（左），在 Erg 中它只是一个对象。

运算符是一种函数，因此它本身也可以绑定到一级对象中的变量。绑定必须用包围。对于<gtr=“4”/>（和<gtr=“5”/>），必须指定<gtr=“6”/>（二元运算）/<gtr=“7”/>（一元运算）以实现唯一化，因为同时存在一元运算符和二元运算符。


```erg
add = `+` # SyntaxError: specify `_+_` or `+_`
add = `_+_`
assert f(1, 2) == 3
assert f("a", "b") == "ab"

g = `*` # OK, this is binary only
assert g(1, 2) == 2
```

但是，请注意，某些称为特殊格式的运算符不能被绑定。


```erg
def = `=` # SyntaxError: cannot bind `=` operator, this is a special form
# NG: def x, 1
function = `->` # SyntaxError: cannot bind `->` operator, this is a special form
# NG: function x, x + 1
```

<p align='center'>
    <a href='./05_builtin_funcs.md'>Previous</a> | <a href='./07_side_effect.md'>Next</a>
</p>

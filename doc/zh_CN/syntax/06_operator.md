# 运算符

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/06_operator.md%26commit_hash%3D20aa4f02b994343ab9600317cebafa2b20676467)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/06_operator.md&commit_hash=20aa4f02b994343ab9600317cebafa2b20676467)

运算符是表示操作的符号。操作数是运算符(左)右侧的东西

运算符是一种函数，因此它们本身就是可以绑定到变量的一流对象。绑定时，需要用```括起来
对于`+`(和`-`)，有一元和二元运算符，所以必须指定`_+_`(二元运算)/`+_`(一元运算)

```python,compile_fail
add = `+` # 语法错误: 指定 `_+_` 或 `+_`
```

```python
add=`_+_`
assert f(1, 2) == 3
assert f("a", "b") == "ab"

mul = `*` # OK, 这只是二进制
assert mul(1, 2) == 2
```

一些称为特殊形式的基本运算符不能被绑定

```python,compile_fail
def = `=` # 语法错误: 无法绑定 `=` 运算符，这是一种特殊形式
# NG: def x, 1
function = `->` # 语法错误: 无法绑定 `->` 运算符，这是一种特殊形式
# NG: function x, x + 1
```

<p align='center'>
    <a href='./05_builtin_funcs.md'>上一页</a> | <a href='./07_side_effect.md'>下一页</a>
</p>

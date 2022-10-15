# 運算符

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/06_operator.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/06_operator.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

運算符是表示操作的符號。操作數是運算符(左)右側的東西

運算符是一種函數，因此它們本身就是可以綁定到變量的一流對象。綁定時，需要用```括起來
對于`+`(和`-`)，有一元和二元運算符，所以必須指定`_+_`(二元運算)/`+_`(一元運算)

```python
add = `+` # 語法錯誤: 指定 `_+_` 或 `+_`
add=`_+_`
assert f(1, 2) == 3
assert f("a", "b") == "ab"

g = `*` # OK, 這只是二進制
assert g(1, 2) == 2
```

一些稱為特殊形式的基本運算符不能被綁定

```python
def = `=` # 語法錯誤: 無法綁定 `=` 運算符，這是一種特殊形式
# NG: def x, 1
function = `->` # 語法錯誤: 無法綁定 `->` 運算符，這是一種特殊形式
# NG: function x, x + 1
```

<p align='center'>
    <a href='./05_builtin_funcs.md'>上一頁</a> | <a href='./07_side_effect.md'>下一頁</a>
</p>

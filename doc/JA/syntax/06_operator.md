# 演算子

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/06_operator.md%26commit_hash%3Deccd113c1512076c367fb87ea73406f91ff83ba7)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/06_operator.md&commit_hash=eccd113c1512076c367fb87ea73406f91ff83ba7)

演算子(オペレーター)は、演算を表す記号です。被演算子(オペランド)は演算子の(左)右にあるもので、Ergでは専らオブジェクトです。

演算子は関数の一種であり、したがってそれ自体も第一級オブジェクトで変数に束縛できます。束縛の際は``で囲む必要があります。
`+`(と`-`)については、単項演算子と二項演算子の両方が存在するため、一意化するために`_+_`(二項演算)/`+_`(単項演算)のどちらかを指定する必要があります。

```erg
add = `+` # SyntaxError: specify `_+_` or `+_`
add = `_+_`
assert f(1, 2) == 3
assert f("a", "b") == "ab"

g = `*` # OK, this is binary only
assert g(1, 2) == 2
```

ただし、特殊形式と呼ばれる一部の演算子は束縛できないことに注意してください。

```erg
def = `=` # SyntaxError: cannot bind `=` operator, this is a special form
# NG: def x, 1
function = `->` # SyntaxError: cannot bind `->` operator, this is a special form
# NG: function x, x + 1
```

<p align='center'>
    <a href='./05_builtin_funcs.md'>Previous</a> | <a href='./07_side_effect.md'>Next</a>
</p>

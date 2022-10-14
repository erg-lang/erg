# 类型绑定

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/19_bound.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/19_bound.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

类型边界为类型规范添加条件。实现这一点的函数是守卫(守卫子句)
此功能可用于函数签名、匿名函数签名以及筛选类型
守卫写在返回类型之后

## 谓词

您可以使用返回 `Bool` 的表达式(谓词表达式)指定变量满足的条件
只能使用 [值对象](./08_value.md) 和运算符。未来版本可能会支持编译时函数

```python
f a: [T; N] | T, N, N > 5 = ...
g a: [T; N | N > 5] | T, N = ...
Odd = {I: Int | I % 2 == 1}
R2Plus = {(L, R) | L, R: Ratio; L > 0 and R > 0}
GeneralizedOdd = {I | U; I <: Div(Nat, U); I % 2 == 0}
```
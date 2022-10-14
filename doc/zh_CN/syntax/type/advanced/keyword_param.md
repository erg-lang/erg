# 带有关键字参数的函数类型

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/advanced/keyword_param.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced/keyword_param.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

```python
h(f) = f(y: 1, x: 2)
h: |T: type|((y: Int, x: Int) -> T) -> T
```

带有关键字参数的函数的子类型化规则如下

```python
((x: T, y: U) -> V) <: ((T, U) -> V) # x, y 为任意关键字参数
((y: U, x: T) -> V) <: ((x: T, y: U) -> V)
((x: T, y: U) -> V) <: ((y: U, x: T) -> V)
```

这意味着可以删除或替换关键字参数
但是你不能同时做这两件事
也就是说，您不能将 `(x: T, y: U) -> V` 转换为 `(U, T) -> V`
请注意，关键字参数仅附加到顶级元组，而不附加到数组或嵌套元组

```python
Valid: [T, U] -> V
Invalid: [x: T, y: U] -> V
Valid: (x: T, ys: (U,)) -> V
Invalid: (x: T, ys: (y: U,)) -> V
```
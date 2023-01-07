# 默认参数

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/advanced/default_param.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced/default_param.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

首先，让我们看一个使用默认参数的示例

```python
f: (Int, Int, z := Int) -> Int
f(x, y, z := 0) = x + y + z

g: (Int, Int, z := Int, w := Int) -> Int
g(x, y, z := 0, w := 1) = x + y + z + w

fold: ((Int, Int) -> Int, [Int], acc := Int) -> Int
fold(f, [], acc) = acc
fold(f, arr, acc := 0) = fold(f, arr[1..], f(acc, arr[0]))
assert fold(f, [1, 2, 3]) == 6
assert fold(g, [1, 2, 3]) == 8
```

`:=` 之后的参数是默认参数
子类型规则如下:

```python
((X, y := Y) -> Z) <: (X -> Z)
((X, y := Y, ...) -> Z) <: ((X, ...) -> Z)
```

第一个意味着可以用没有默认参数的函数来识别具有默认参数的函数
第二个意味着可以省略任何默认参数。
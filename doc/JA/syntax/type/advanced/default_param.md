# デフォルト引数付きの関数型

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/advanced/default_param.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced/default_param.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

まず、デフォルト引数の使用例を見る。

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

`:=`以降の引数はデフォルト引数である。
部分型付け規則は以下の通り。

```python
((X, y := Y) -> Z) <: (X -> Z)
((X, y := Y, ...) -> Z) <: ((X, ...) -> Z)
```

1番目は、デフォルト引数のある関数は、ない関数と同一視できる、という意味である。
2番目は、任意のデフォルト引数は省略できる、という意味である。

デフォルト引数の型は、引数を渡した場合と渡さなかった場合で変えることができる。
具体的には、`if`関数の型などが良い例である。

```python
if: |T: Type, U: Type|(then: () -> T, else: () -> U := () -> NoneType) -> T or U
```

`if`関数は、`else`引数が与えられなければ`T or NoneType`を返す。

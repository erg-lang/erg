# キーワード引数付き関数型

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/advanced/keyword_param.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced/keyword_param.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

```python
h(f) = f(y: 1, x: 2)
h: |T: Type|((y: Int, x: Int) -> T) -> T
```

キーワード引数付き関数の部分型付け規則は以下の通り。

```python
((x: T, y: U) -> V) <: ((T, U) -> V)  # x, yは任意のキーワードパラメータ
((y: U, x: T) -> V) <: ((x: T, y: U) -> V)
((x: T, y: U) -> V) <: ((y: U, x: T) -> V)
```

これは、キーワード引数は消去ないし入れ替えができるということを意味する。
しかし、両者を同時に行うことはできない。
すなわち、`(x: T, y: U) -> V`を`(U, T) -> V`にキャストすることはできない。
なお、キーワード引数がつくのはトップレベルのタプル内のみで、配列やネストしたタプルでキーワード引数は付かない。

```python
Valid: [T, U] -> V
Invalid: [x: T, y: U] -> V
Valid: (x: T, ys: (U,)) -> V
Invalid: (x: T, ys: (y: U,)) -> V
```

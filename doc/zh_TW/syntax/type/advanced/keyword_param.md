# 帶有關鍵字參數的函數類型

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/advanced/keyword_param.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced/keyword_param.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

```python
h(f) = f(y: 1, x: 2)
h: |T: type|((y: Int, x: Int) -> T) -> T
```

帶有關鍵字參數的函數的子類型化規則如下

```python
((x: T, y: U) -> V) <: ((T, U) -> V) # x, y 為任意關鍵字參數
((y: U, x: T) -> V) <: ((x: T, y: U) -> V)
((x: T, y: U) -> V) <: ((y: U, x: T) -> V)
```

這意味著可以刪除或替換關鍵字參數
但是你不能同時做這兩件事
也就是說，您不能將 `(x: T, y: U) -> V` 轉換為 `(U, T) -> V`
請注意，關鍵字參數僅附加到頂級元組，而不附加到數組或嵌套元組

```python
Valid: [T, U] -> V
Invalid: [x: T, y: U] -> V
Valid: (x: T, ys: (U,)) -> V
Invalid: (x: T, ys: (y: U,)) -> V
```
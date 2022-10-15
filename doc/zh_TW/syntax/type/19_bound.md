# 類型綁定

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/19_bound.md%26commit_hash%3D412a6fd1ea507a7afa1304bcef642dfe6b3a0872)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/19_bound.md&commit_hash=412a6fd1ea507a7afa1304bcef642dfe6b3a0872)

類型邊界為類型規范添加條件。實現這一點的函數是守衛(守衛子句)
此功能可用于函數簽名、匿名函數簽名以及篩選類型
守衛寫在返回類型之后

## 謂詞

您可以使用返回 `Bool` 的表達式(謂詞表達式)指定變量滿足的條件
只能使用 [值對象](./08_value.md) 和運算符。未來版本可能會支持編譯時函數

```python
f a: [T; N] | T, N, N > 5 = ...
g a: [T; N | N > 5] | T, N = ...
Odd = {I: Int | I % 2 == 1}
R2Plus = {(L, R) | L, R: Ratio; L > 0 and R > 0}
GeneralizedOdd = {I | U; I <: Div(Nat, U); I % 2 == 0}
```
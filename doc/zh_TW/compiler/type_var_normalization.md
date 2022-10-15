# 歸一化

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/compiler/type_var_normalization.md%26commit_hash%3Dd15cbbf7b33df0f78a575cff9679d84c36ea3ab1)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/compiler/type_var_normalization.md&commit_hash=d15cbbf7b33df0f78a575cff9679d84c36ea3ab1)

* Erg 的類型參數規范化基于 SymPy 的簡化函數

例如，當您定義 `concat: |T, M, N|([T; M], [T; N] -> [T; M+N])` 時，您可以匹配類型變量和參數而無需實例化它們.必須作出判斷
平等判斷自然有其局限性，但目前可能的判斷及其方法如下

* 加法/乘法對稱性: 

  `n+m == m+n`

  類型變量被排序和規范化為字符串

* 加法、乘法、減法和除法等價: 

  `n+n == 2*n`

  歸一化為 Σ[c] x == c*x，其中 c 是一個常數
  常量通過將它們放在二進制操作的左側進行標準化

* 雙重表達式的相等性: 

  `n+m+l == m+n+l == l+m+n == ...`
  `n+m*l == m*l+n`

  通過排序歸一化確定
  乘法和除法塊放置在加法和減法的左側。通過比較最左側的類型變量對塊進行排序

* 基本不等式: 

  `n > m -> m + 1 > n`

* 平等: 

  `n >= m 和 m >= n -> m == n`

* 不等式的傳遞性: 

  `n > 0 -> n > -1`
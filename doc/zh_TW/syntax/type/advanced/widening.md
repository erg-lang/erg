# 指示轉換運算符關鍵字: widening

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/advanced/widening.md%26commit_hash%3Db07c17708b9141bbce788d2e5b3ad4f365d342fa)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced/widening.md&commit_hash=b07c17708b9141bbce788d2e5b3ad4f365d342fa)

例如，定義多相關系數如下

```python
ids|T|(x: T, y: T) = x, y
```

分配同一類的一對實例并沒有錯
當您分配另一個具有包含關系的類的實例對時，它會向上轉換為較大的類并成為相同的類型
另外，很容易理解，如果分配了另一個不在包含關系中的類，就會發生錯誤

```python
assert ids(1, 2) == (1, 2)
assert ids(1, 2.0) == (1.0, 2.0)
ids(1, "a") # TypeError
```

現在，具有不同派生類型的類型呢?

```python
i: Int or Str
j: Int or NoneType
ids(i, j) # ?
```

在解釋這一點之前，我們必須關注 Erg 的類型系統實際上并不關注(運行時)類這一事實

```python
1: {__valueclass_tag__ = Phantom Int}
2: {__valueclass_tag__ = Phantom Int}
2.0: {__valueclass_tag__ = Phantom Ratio}
"a": {__valueclass_tag__ = Phantom Str}
ids(1, 2): {__valueclass_tag__ = Phantom Int} and {__valueclass_tag__ = Phantom Int} == {__valueclass_tag__ = Phantom Int}
ids(1, 2.0): {__valueclass_tag__ = Phantom Int} and {__valueclass_tag__ = Phantom Ratio} == {__valueclass_tag__ = Phantom Ratio} # Int < Ratio
ids(1, "a"): {__valueclass_tag__ = Phantom Int} and {__valueclass_tag__ = Phantom Str} == Never # 類型錯誤
```

我看不到該類，因為它可能無法準確看到，因為在 Erg 中，對象的類屬于運行時信息
例如，一個`Int`或Str`類型的對象的類是`Int`或`Str`，但你只有通過執行才能知道它是哪一個
當然，`Int` 類型的對象的類被定義為 `Int`，但是在這種情況下，從類型系統中可見的是 `Int` 的結構類型 `{__valueclass_tag__ = Int}`

現在讓我們回到另一個結構化類型示例。總之，上述代碼將導致類型錯誤，因為類型不匹配
但是，如果您使用類型注釋進行類型擴展，編譯將通過

```python
i: Int or Str
j: Int or NoneType
ids(i, j) # 類型錯誤: i 和 j 的類型不匹配
# 提示: 嘗試擴大類型(例如 ids<Int or Str or NoneType>)
ids<Int or Str or NoneType>(i, j) # OK
```

`A 和 B` 有以下可能性

* `A and B == A`: 當`A <: B`或`A == B`時
* `A and B == B`: 當 `A :> B` 或 `A == B` 時
* `A and B == {}`: 當 `!(A :> B)` 和 `!(A <: B)` 時

`A 或 B` 具有以下可能性

* `A 或 B == A`: 當`A :> B` 或`A == B` 時
* `A or B == B`: 當`A <: B`或`A == B`時
* `A 或 B` 是不可約的(獨立類型): 如果 `!(A :> B)` 和 `!(A <: B)`

## 子程序定義中的類型擴展

如果返回類型不匹配，Erg 默認會出錯

```python
parse_to_int s: Str =
    if not s.is_numeric():
        do parse_to_int::return error("not numeric")
... # 返回 Int 對象
# 類型錯誤: 返回值類型不匹配
# 3 | 做 parse_to_int::return error("not numeric")
# └─ Error
# 4 | ...
# └ Int
```

為了解決這個問題，需要將返回類型顯式指定為 Or 類型

```python
parse_to_int(s: Str): Int or Error =
    if not s.is_numeric():
        do parse_to_int::return error("not numeric")
    ... # 返回 Int 對象
```

這是設計使然，這樣您就不會無意中將子例程的返回類型與另一種類型混合
但是，如果返回值類型選項是具有包含關系的類型，例如 `Int` 或 `Nat`，它將與較大的對齊。
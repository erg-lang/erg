# 代數類型

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/13_algebraic.md%26commit_hash%3Dc120700585fdb1d655255c8e2817bb13cc8d369e)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/13_algebraic.md&commit_hash=c120700585fdb1d655255c8e2817bb13cc8d369e)

代數類型是通過將類型視為代數來操作類型而生成的類型。
它們處理的操作包括Union、Intersection、Diff、Complement等。
普通類只能進行Union，其他操作會導致類型錯誤。

## 聯合(Union)

聯合類型可以為類型提供多種可能性。 顧名思義，它們是由"或"運算符生成的。
一個典型的 Union 是 `Option` 類型。 `Option` 類型是 `T 或 NoneType` 補丁類型，主要表示可能失敗的值。

```python
IntOrStr = Int or Str
assert dict.get("some key") in (Int or NoneType)

# 隱式變為 `T != NoneType`
Option T = T or NoneType
```

## 路口

交集類型是通過將類型與 `and` 操作組合得到的。

```python
Num = Add and Sub and Mul and Eq
```

如上所述，普通類不能與"and"操作結合使用。 這是因為實例只屬于一個類。

## 差異

Diff 類型是通過 `not` 操作獲得的。
最好使用 `and not` 作為更接近英文文本的符號，但建議只使用 `not`，因為它更適合與 `and` 和 `or` 一起使用。

```python
CompleteNum = Add and Sub and Mul and Div and Eq and Ord
Num = CompleteNum not Div not Ord

True = Bool not {False}
OneTwoThree = {1, 2, 3, 4, 5, 6} - {4, 5, 6, 7, 8, 9, 10}
```

## 補充

補碼類型是通過 `not` 操作得到的，這是一個一元操作。 `not T` 類型是 `{=} not T` 的簡寫。
類型為"非 T"的交集等價于 Diff，類型為"非 T"的 Diff 等價于交集。
但是，不推薦這種寫法。

```python
# 非零數類型的最簡單定義
NonZero = Not {0}
# 不推薦使用的樣式
{True} == Bool and not {False} # 1 == 2 + - 1
Bool == {True} not not {False} # 2 == 1 - -1
```

## 真代數類型

有兩種代數類型：可以簡化的表觀代數類型和不能進一步簡化的真實代數類型。
"表觀代數類型"包括 Enum、Interval 和 Record 類型的 `or` 和 `and`。
這些不是真正的代數類型，因為它們被簡化了，并且將它們用作類型說明符將導致警告； 要消除警告，您必須簡化它們或定義它們的類型。

```python
assert {1, 2, 3} or {2, 3} == {1, 2, 3}
assert {1, 2, 3} and {2, 3} == {2, 3}
assert -2..-1 or 1..2 == {-2, -1, 1, 2}

i: {1, 2} or {3, 4} = 1 # 類型警告：{1, 2} 或 {3, 4} 可以簡化為 {1, 2, 3, 4}
p: {x = Int, ...} and {y = Int; ...} = {x = 1; y = 2; z = 3}
# 類型警告：{x = Int, ...} 和 {y = Int; ...} 可以簡化為 {x = Int; y = 整數； ...}

Point1D = {x = Int; ...}
Point2D = Point1D and {y = Int; ...} # == {x = Int; y = Int; ...}
q: Point2D = {x = 1; y = 2; z = 3}
```

真正的代數類型包括類型"或"和"與"。 類之間的"或"等類屬于"或"類型。

```python
assert Int or Str == Or(Int, Str)
assert Int and Marker == And(Int, Marker)
```

Diff, Complement 類型不是真正的代數類型，因為它們總是可以被簡化。

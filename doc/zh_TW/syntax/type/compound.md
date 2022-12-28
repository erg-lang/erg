# 複合型

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/compound.md%26commit_hash%3D96b113c47ec6ca7ad91a6b486d55758de00d557d)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced.md&commit_hash=96b113c47ec6ca7ad91a6b486d55758de00d557d)

## 元組類型

```erg
(), (X,), (X, Y), (X, Y, Z), ...
```

元組具有長度和內部類型的子類型化規則
對於任何元組`T`，`U`，以下成立

```erg
* T <: () (單位規則)
* forall N in 0..<Len(T) (Len(T) <= Len(U)), U.N == T.N => U <: T (遺忘規則)
```

例如，`(Int, Str, Bool) <: (Int, Str)`
但是，這些規則不適用於函數類型的（可見）元組部分。這是因為這部分實際上不是元組

```erg
(Int, Int) -> Int !<: (Int,) -> Int
```

還有單位類型的返回值可以忽略，但是其他元組類型的返回值不能忽略

## 數組類型

```erg
[], [X; 0], [X; 1], [X; 2], ..., [X; _] == [X]
```

數組和元組存在類似的子類型化規則

```erg
* T <: [] (單位規則)
* forall N in 0..<Len(T) (Len(T) <= Len(U)), U[N] == T[N] => U <: T (遺忘規則)
```

像下面這樣的數組不是有效類型。這是一個刻意的設計，強調陣列元素是同質化的

```erg
[Int, Str]
```

因此，每個元素的詳細信息都會丟失。使用篩模來保存它

```erg
a = [1, "a"]: {A: [Int or Str; 2] | A[0] == Int}
a[0]: Int
```

## 設置類型

```erg
{}, {X}, ...
```

集合類型本身不攜帶長度信息。這是因為元素的重複項在集合中被消除，但重複項通常無法在編譯時確定。首先，長度信息在集合中沒有多大意義

`{}`是空集，是所有類型的子類型

## 詞典類型

```erg
{:}, {X: Y}, {X: Y, Z: W}, ...
```

## 記錄類型

```erg
{=}, {i = Int}, {i = Int; j = Int}, {.i = Int; .j = Int}, ...
```

具有私有屬性的類型和具有公共屬性的類型之間沒有子類型關係，但它們可以通過`.Into`相互轉換。

```erg
r = {i = 1}.Into {.i = Int}
assert r.i == 1
```

## 函数類型

```erg
() -> ()
Int -> Int
(Int, Str) -> Bool
(x: Int, y: Int) -> Int
(x := Int, y := Int) -> Int
(...objs: Obj) -> Str
(Int, Ref Str!) -> Int
|T: Type|(x: T) -> T
|T: Type|(x: T := NoneType) -> T # |T: Type|(x: T := X, y: T := Y) -> T (X != Y) is invalid
```

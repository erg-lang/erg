# 復合型

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/20_compound.md%26commit_hash%3Dc6eb78a44de48735213413b2a28569fdc10466d0)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/20_compound.md&commit_hash=c6eb78a44de48735213413b2a28569fdc10466d0)

## 元組類型

```erg
(), (X,), (X, Y), (X, Y, Z), ...
```

元組具有長度和內亞型的子類型化規則
對于任何元組`T`，`U`，以下成立

```erg
* T <: () (單位規則)
* forall N in 0..<Len(T) (Len(T) <= Len(U)), U.N == T.N => U <: T (遺忘規則)
```

例如，`(Int, Str, Bool) <: (Int, Str)`
但是，這些規則不適用于函數類型的（可見）元組部分。這是因為這部分實際上不是元組

```erg
(Int, Int) -> Int !<: (Int,) -> Int
```

還有單位類型的返回值可以忽略，但是其他tuple類型的返回值不能忽略

## 配列型

```erg
[], [X; 0], [X; 1], [X; 2], ..., [X; _] == [X]
```

數組和元組存在類似的子類型化規則

```erg
* T <: [] (單位規則)
* forall N in 0..<Len(T) (Len(T) <= Len(U)), U[N] == T[N] => U <: T (遺忘規則)
```

像下面這樣的數組不是有效類型。 這是一個刻意的設計，強調陣列元素是同質化的

```erg
[Int, Str]
```

因此，每個元素的詳細信息都會丟失。可以使用細化類型。

```erg
a = [1, "a"]: {A: [Int or Str; 2] | A[0] == Int}
a[0]: Int
```

## 設置類型

```erg
{}, {X; _}, ...
```

Set types have length information, but mostly useless. This is because duplicate elements are eliminated in sets, but duplicate elements cannot generally be determined at compile time.
In the first place, the length of the information is not very meaningful in a Set.

`{}`是一個空集合，是擁有類型的子類型. Note that `{X}` is not a set type, but a type that contains only one constant `X`.

## 詞典類型

```erg
{:}, {X: Y}, {X: Y, Z: W}, ...
```

All dict types are subtypes of `Dict K, V`. `{X: Y} <: Dict X, Y` and `{X: Y, Z: W} <: Dict X or Z, Y or W`.

## 記錄類型

```erg
{=}, {i = Int}, {i = Int; j = Int}, {.i = Int; .j = Int}, ...
```

A private record type is a super type of a public record type.

e.g. `{.i = Int} <: {i = Int}`

## 函數類型

```erg
() -> ()
Int -> Int
(Int, Str) -> Bool
# named parameter
(x: Int, y: Int) -> Int
# default parameter
(x := Int, y := Int) -> Int
# variable-length parameter
(*objs: Obj) -> Str
(Int, Ref Str!) -> Int
# qualified parameter
|T: Type|(x: T) -> T
# qualified parameter with default type
|T: Type|(x: T := NoneType) -> T # |T: Type|(x: T := X, y: T := Y) -> T (X != Y) is invalid
```

## Bound Method Type

```erg
Int.() -> Int
Int.(other: Int) -> Int
# e.g. 1.__add__: Int.(Int) -> Int
```

The type `C.(T) -> U` is a subtype of `T -> U`. They are almost the same, but ``C.(T) -> U`` is the type of a method whose receiver type is `C`, and the receiver is accessible via an attribute `__self__`.

<p align='center'>
    <a href='./19_bound.md'>上一頁</a> | 下一頁
</p>
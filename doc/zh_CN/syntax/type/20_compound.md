# 复合型

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/20_compound.md%26commit_hash%3Dc6eb78a44de48735213413b2a28569fdc10466d0)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/20_compound.md&commit_hash=c6eb78a44de48735213413b2a28569fdc10466d0)

## 元组类型

```erg
(), (X,), (X, Y), (X, Y, Z), ...
```

元组内部有长度和类型的子类型规则。
对于任何元组`T`，`U`，以下成立

```erg
* T <: () (单位规则)
* forall N in 0..<Len(T) (Len(T) <= Len(U)), U.N == T.N => U <: T (遗忘规则)
```

例如，`(Int, Str, Bool) <: (Int, Str)`
但是，这些规则不适用于函数类型的（可见）元组部分。这是因为这部分实际上不是元组

```erg
(Int, Int) -> Int !<: (Int,) -> Int
```

还有单位类型的返回值可以忽略，但是其他tuple类型的返回值不能忽略

## 配列型

```erg
[], [X; 0], [X; 1], [X; 2], ..., [X; _] == [X]
```

数组和元组存在类似的子类型化规则

```erg
* T <: [] (单位规则)
* forall N in 0..<Len(T) (Len(T) <= Len(U)), U[N] == T[N] => U <: T (遗忘规则)
```

像下面这样的数组不是有效的类型。这是一种有意的设计，目的是强调数组的元素是均质的。

```erg
[Int, Str]
```

因此，每个元素的详细信息都会丢失。为了保持这一点，可以使用细化类型

```erg
a = [1, "a"]: {A: [Int or Str; 2] | A[0] == Int}
a[0]: Int
```

## 设置类型

```erg
{}, {X; _}, ...
```

Set types have length information, but mostly useless. This is because duplicate elements are eliminated in sets, but duplicate elements cannot generally be determined at compile time.
In the first place, the length of the information is not very meaningful in a Set.

`{}`是一个空集合，是拥有类型的子类型. Note that `{X}` is not a set type, but a type that contains only one constant `X`.

## 词典类型

```erg
{:}, {X: Y}, {X: Y, Z: W}, ...
```

All dict types are subtypes of `Dict K, V`. `{X: Y} <: Dict X, Y` and `{X: Y, Z: W} <: Dict X or Z, Y or W`.

## 记录类型

```erg
{=}, {i = Int}, {i = Int; j = Int}, {.i = Int; .j = Int}, ...
```

A private record type is a super type of a public record type.

e.g. `{.i = Int} <: {i = Int}`

## 函数类型

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
    <a href='./19_bound.md'>上一页</a> | 下一页
</p>
# 复合型

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/compound.md%26commit_hash%3D96b113c47ec6ca7ad91a6b486d55758de00d557d)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced.md&commit_hash=96b113c47ec6ca7ad91a6b486d55758de00d557d)

## 元组类型

```erg
(), (X,), (X, Y), (X, Y, Z), ...
```

元组具有长度和内部类型的子类型化规则
对于任何元组“T”，“U”，以下成立

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

像下面这样的数组不是有效类型。 这是一个刻意的设计，强调阵列元素是同质化的

```erg
[Int, Str]
```

因此，每个元素的详细信息都会丢失。 使用筛模来保存它

```erg
a = [1, "a"]: {A: [Int or Str; 2] | A[0] == Int}
a[0]: Int
```

## 设置类型

```erg
{}, {X}, ...
```

集合类型本身不携带长度信息。这是因为元素的重复项在集合中被消除，但重复项通常无法在编译时确定。首先，长度信息在集合中没有多大意义

`{}`是一个空集合，是拥有类型的子类型

## 词典类型

```erg
{:}, {X: Y}, {X: Y, Z: W}, ...
```

## 记录类型

```erg
{=}, {i = Int}, {i = Int; j = Int}, {.i = Int; .j = Int}, ...
```

具有私有属性的类型和具有公共属性的类型之间没有子类型关系，但它们可以通过`.Into`相互转换

```erg
r = {i = 1}.Into {.i = Int}
assert r.i == 1
```

## 函数类型

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

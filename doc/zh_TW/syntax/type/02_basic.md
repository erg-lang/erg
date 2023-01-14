# 基本語法

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/02_basic.md%26commit_hash%3Df4fb25b4004bdfa96d2149fac8c4e40b84e8a45f)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/02_basic.md&commit_hash=f4fb25b4004bdfa96d2149fac8c4e40b84e8a45f)

## 類型規范

在 Erg 中，可以在 `:` 之后指定變量的類型，如下所示。這可以與作業同時完成

```python
i: Int # 將變量 i 聲明為 Int 類型
i: Int = 1
j = 1 # 類型說明可以省略
```

您還可以指定普通表達式的類型

```python
i = 1: Int
f([1, "a"]: [Int or Str])
```

對于簡單的變量賦值，大多數類型說明可以省略
在定義子例程和類型時，類型規范更有用

```python
# 參數的類型規范
f x, y: Array Int = ...
T X, Y: Array Int = ...
```

請注意，在上述情況下，`x, y` 都是 `Array Int`

```python
# 大寫變量的值必須是常量表達式
f X: Int = X
```

或者，如果你不需要關于類型參數的完整信息，你可以用 `_` 省略它

```python,compile_fail
g v: [T; _] = ...
```

但是請注意，類型規范中的 `_` 意味著 `Object`

```python
f x: _, y: Int = x + y # 類型錯誤: Object 和 Int 之間沒有實現 +
```

## 子類型規范

除了 `:`(類型聲明運算符)，Erg 還允許您使用 `<:`(部分類型聲明運算符)來指定類型之間的關系
`<:` 的左邊只能指定一個類。使用 `Subtypeof` 或類似的運算符來比較結構類型

這也經常在定義子例程或類型時使用，而不是簡單地指定變量

```python
# 參數的子類型規范
f X <: T = ...

# 所需屬性的子類型規范(.Iterator 屬性必須是 Iterator 類型的子類型)
Iterable T = Trait {
    .Iterator = {Iterator} # {Iterator} == {I: Type | I <: Iterator}
    .iter = Self.() -> Self.Iterator T
    ...
}
```

也可以在定義類時使用子類型規范來靜態檢查該類是否是指定類型的子類型

```python
# C 類是 Show 的子類型
C = Class Object, Impl := Show
C.show self = ... # 顯示所需的屬性
```

您也可以僅在特定情況下指定子類型

```python
K T: Eq
K Int <: Show and Eq
K T = Class Object
K(T).
    `==` self, other = ...
K(Int).
    show self = ...
```

實現結構類型時建議使用子類型規范
這是因為，由于結構子類型的性質，拼寫錯誤或類型規范錯誤在實現所需屬性時不會導致錯誤

```python
C = Class Object
C.shoe self = ... # Show 由于 Typo 沒有實現(它被認為只是一種獨特的方法)
```

## 屬性定義

只能在模塊中為Trait和類定義屬性

```python
C = Class()
C.pub_attr = "this is public"
C::private_attr = "this is private"

c = C.new()
assert c.pub_attr == "this is public"
```

定義批處理定義的語法稱為批處理定義，其中在 `C.` 或 `C::` 之后添加換行符，并且定義在縮進下方組合在一起

```python
C = Class()
C.pub1 = ...
C.pub2 = ...
C::priv1 = ...
C::priv2 = ...
# 相當于
C = Class()
C.
    pub1 = ...
    C. pub2 = ...
C::
    priv1 = ...
    priv2 = ...
```

## 別名

類型可以有別名。這允許縮短長類型，例如記錄類型

```python
Id = Int
Point3D = {x = Int; y = Int; z = Int}
IorS = Int or Str
Vector = Array Int
```

此外，當顯示錯誤時，如果定義了復合類型(在上面的示例中，右側類型不是第一個類型)，編譯器將為它們使用別名

但是，每個模塊只允許一個相同類型的別名，多個別名將導致警告
這意味著應將具有不同用途的類型定義為單獨的類型
目的還在于防止在已經具有別名的類型之上添加別名

```python,compile_warn
Id = Int
UserId = Int # 類型警告: 重復別名: Id 和 UserId

Ids = Array Id
Ints = Array Int # 類型警告: 重復別名: Isd 和 Ints

IorS = Int or Str
IorSorB = IorS or Bool
IorSorB_ = Int or Str or Bool # 類型警告: 重復別名: IorSorB 和 IorSorB_

Point2D = {x = Int; y = Int}
Point3D = {.... Point2D; z = Int}
Point = {x = Int; y = Int; z = Int} # 類型警告: 重復別名: Point3D 和 Point
```

<p align='center'>
    <a href='./01_type_system.md'>上一頁</a> | <a href='./03_trait.md'>下一頁</a>
</p>

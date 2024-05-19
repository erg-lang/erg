# Erg 的類型系統

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/01_type_system.md%26commit_hash%3Dc6eb78a44de48735213413b2a28569fdc10466d0)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/01_type_system.md&commit_hash=c6eb78a44de48735213413b2a28569fdc10466d0)

下面簡單介紹一下 Erg 的類型系統。詳細信息在其他部分進行說明

## 如何定義

Erg 的獨特功能之一是(普通)變量、函數(子例程)和類型(Kind)定義之間的語法沒有太大區別。所有都是根據普通變量和函數定義的語法定義的

```python
f i: Int = i + 1
f # <函數 f>
f(1) # 2
f.method self = ... # 語法錯誤: 無法為子例程定義方法

T I: Int = {...}
T # <kind 'T'>
T(1) # 類型 T(1)
T.method self = ...
D = Class {private = Int; .public = Int}
D # <類 'D'>
o1 = {private = 1; .public = 2} # o1 是一個不屬于任何類的對象
o2 = D.new {private = 1; .public = 2} # o2 是 D 的一個實例
o2 = D.new {.public = 2} # 初始化錯誤: 類 'D' 需要屬性 'private'(: Int) 但未定義
```

## Classification

Erg 中的所有對象都是強類型的
頂層類型是`{=}`，實現了`__repr__`、`__hash__`、`clone`等(不是必須的方法，這些屬性不能被覆蓋)
Erg 的類型系統包含結構子類型 (SST)。該系統類型化的類型稱為結構類型
結構類型主要分為三種: Attributive(屬性類型)、Refinement(細化類型)和Algebraic(代數類型)

|           | Record      | Enum       | Interval       | Union       | Intersection | Diff         |
| --------- | ----------- | ---------- | -------------- | ----------- | ------------ | ------------ |
| kind      | Attributive | Refinement | Refinement     | Algebraic   | Algebraic    | Algebraic    |
| generator | record      | set        | range operator | or operator | and operator | not operator |

也可以使用名義子類型(NST)，將 SST 類型轉換為 NST 類型稱為類型的名義化。結果類型稱為名義類型
在 Erg 中，名義類型是類和Trait。當我們簡單地說類/Trait時，我們通常指的是記錄類/Trait

|     | Type           | Abstraction      | Subtyping procedure |
| --- | -------------- | ---------------- | ------------------- |
| NST | NominalType    | Trait            | Inheritance         |
| SST | StructuralType | Structural Trait | (Implicit)          |

整個名義類型的類型(`NominalType`)和整個結構類型的類型(`StructuralType`)是整個類型(`Type`)的類型的子類型

Erg 可以將參數(類型參數)傳遞給類型定義。帶有類型參數的 `Option`、`List` 等稱為多項式類型。這些本身不是類型，但它們通過應用參數成為類型。諸如 `Int`、`Str` 等沒有參數的類型稱為簡單類型(標量類型)

一個類型可以看成一個集合，并且存在包含關系。例如，"Num"包含"Add"、"Sub"等，"Int"包含"Nat"
所有類的上類是`Object == Class {:}`，所有類型的下類是`Never == Class {}`。這在下面描述

## 類型

像 `List T` 這樣的類型可以看作是 `Type -> Type` 類型的函數，它以 `T` 類型為參數并返回 `List T` 類型(在類型論中也稱為 Kind)。像 `List T` 這樣的類型專門稱為多態類型，而 `List` 本身稱為一元 Kind

已知參數和返回類型的函數的類型表示為`(T, U) -> V`。如果要指定同一類型的整個雙參數函數，可以使用 `|T| (T, T) -> T`，如果要指定整個 N 參數函數，可以使用 `Func N`。但是，`Func N` 類型沒有關于參數數量或其類型的信息，因此所有返回值在調用時都是`Obj` 類型

`Proc` 類型表示為 `() => Int` 等等。此外，`Proc` 類型實例的名稱必須以 `!` 結尾

`Method` 類型是一個函數/過程，其第一個參數是它所屬的對象 `self`(通過引用)。對于依賴類型，也可以在應用方法后指定自己的類型。這是 `T!(!N)` 類型和 `T!(N ~> N-1)。() => Int` 等等

Erg 的數組(List)就是 Python 所說的列表。`[詮釋; 3]`是一個數組類，包含三個`Int`類型的對象

> __Note__: `(Type; N)` 既是類型又是值，所以可以這樣使用
>
> ```python.
> Types = (Int, Str, Bool)
>
> for! Types, T =>
>     print! T
> # Int Str Bool
> a: Types = (1, "aaa", True)
> ```

```python
pop|T, N|(l: [T; N]): ([T; N-1], T) =
    [*l, last] = l
    (l, last)

lpop|T, N|(l: [T; N]): (T, [T; N-1]) =
    [first, *l] = l
    (first, l)
```

以 `!` 結尾的類型可以重寫內部結構。例如，`[T; !N]` 類是一個動態數組
要從"T"類型的對象創建"T!"類型的對象，請使用一元運算符"!"

```python
i: Int! = !1
i.update! i -> i + 1
assert i == 2
arr = [1, 2, 3]
arr.push! 4 # 導入錯誤
mut_arr = [1, 2, 3].into [Int; !3]
mut_arr.push4
assert mut_arr == [1, 2, 3, 4].
```

## 類型定義

類型定義如下

```python
Point2D = {.x = Int; .y = Int}
```

請注意，如果從變量中省略 `.`，它將成為類型中使用的私有變量。但是，這也是必需的屬性
由于類型也是對象，因此類型本身也有屬性。這樣的屬性稱為類型屬性。在類的情況下，它們也稱為類屬性

## 數據類型

如前所述，Erg 中的"類型"大致表示一組對象

下面是 `Add` 類型的定義，需要 `+`(中間運算符)。`R, O` 是所謂的類型參數，可以是真正的類型(類)，例如 `Int` 或 `Str`。在其他語言中，類型參數被賦予特殊的符號(泛型、模板等)，但在 Erg 中，它們可以像普通參數一樣定義
類型參數也可以用于類型對象以外的類型。例如數組類型`[Int; 3]` 是 `List Int, 3` 的語法糖。如果類型實現重疊，用戶必須明確選擇一個

```python
Add R = Trait {
    .AddO = Type
    . `_+_` = Self.(R) -> Self.AddO
}
```

.`_+_`是Add.`_+_`的縮寫。前綴運算符 .`+_` 是 `Num` 類型的方法

```python
Num = Add and Sub and Mul and Eq
NumImpl = Patch Num
NumImpl.
    `+_`(self): Self = self
    ...
```

多態類型可以像函數一樣對待。通過將它們指定為 `Mul Int、Str` 等，它們可以是單態的(在許多情況下，它們是用實際參數推斷出來的，而沒有指定它們)

```python
1 + 1
`_+_` 1, 1
Nat.`_+_` 1, 1
Int.`_+_` 1, 1
```

前四行返回相同的結果(準確地說，底部的返回 `Int`)，但通常使用頂部的
`Ratio.`_+_`(1, 1)` 將返回 `2.0` 而不會出錯
這是因為 `Int <: Ratio`，所以 `1` 向下轉換為 `Ratio`
但這不是演員

```python
i = 1
if i: # 類型錯誤: i: Int 不能轉換為 Bool，請改用 Int.is_zero()
    log "a"
    log "b"
```

這是因為 `Bool <: Int` (`True == 1`, `False == 0`)。轉換為子類型通常需要驗證

## 類型推理系統

Erg 使用靜態鴨子類型，因此幾乎不需要顯式指定類型

```python
f x, y = x + y
```

在上面的代碼中，帶有 `+` 的類型，即 `Add` 是自動推斷的； Erg 首先推斷出最小的類型。如果`f 0, 1`，它將推斷`f x: {0}，y: {1}`，如果`n: Nat; f n, 1`，它會推斷`f x: Nat, y: {1}`。最小化之后，增加類型直到找到實現。在 `{0}, {1}` 的情況下，`Nat` 與 `Nat` 是單態的，因為 `Nat` 是具有 `+` 實現的最小類型
如果是 `{0}, {-1}`，它與 `Int` 是單態的，因為它不匹配 `Nat`。如果子類型和父類型之間沒有關系，則首先嘗試具有最低濃度(實例數)(或者在多態類型的情況下參數更少)的那個
`{0}` 和 `{1}` 是枚舉類型，它們是部分類型，例如 `Int` 和 `Nat`
例如，可以為枚舉類型指定名稱和請求/實現方法。在有權訪問該類型的命名空間中，滿足請求的對象可以使用實現方法

```python
Binary = Patch {0, 1}
Binary.
    # self 包含一個實例。在此示例中，為 0 或 1
    # 如果你想重寫self，你必須追加！ 必須添加到類型名稱和方法名稱
    is_zero(self) = match self:
        0 -> True
        1 -> False # 你也可以使用 _ -> False
    is_one(self) = not self.is_zero()
    to_bool(self) = match self:
        0 -> False
        1 -> True
```

此后，代碼"0.to_bool()"是可能的(盡管"0 as Bool == False"是內置定義的)
這是一個實際上可以重寫 `self` 的類型的示例，如代碼所示

```python
Binary! = Patch {0, 1}!
Binary!
    switch! ref! self = match! self:
        0 => self = 1
        1 => self = 0

b = !1
b.switch!()
print! b # => 0
```

## 結構類型(匿名類型)

```python
Binary = {0, 1}
```

上面代碼中的 `Binary` 是一個類型，其元素是 `0` 和 `1`。它也是 `Int` 類型的子類型，它同時具有 `0` 和 `1`
像 `{}` 這樣的對象本身就是一種類型，可以在分配或不分配給上述變量的情況下使用
這樣的類型稱為結構類型。當我們想強調它作為后者而不是類(命名類型)的用途時，它也被稱為未命名類型。`{0, 1}`這樣的結構類型稱為枚舉類型，還有區間類型、記錄類型等

### 類型標識

無法指定以下內容。例如，您不能指定 `Int` 和 `Int` 和 `Int` 和 `Int` 和 `Int` 和 `Int`
例如，`Int`和`Str`都是`Add`，但是`Int`和`Str`不能相加

```python
add l: Add, r: Add =
    l + r # 類型錯誤: `_+_` 沒有實現: |T, U <: Add| (T, U) -> <失敗>
```

此外，下面的類型 `A` 和 `B` 不被認為是同一類型。但是，類型"O"被認為是匹配的

```python
... |R1, R2, O, A <: Add(R1, O); B <: Add(R2, O)|
```

<p align='center'>
    上一頁 | <a href='./02_basic.md'>下一頁</a>
</p>

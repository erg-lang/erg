# 變性(逆變與協變 Variance)

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/advanced/variance.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced/variance.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

Erg 可以對多態類型進行子類型化，但有一些注意事項

首先，考慮普通多態類型的包含關系。一般來說，有一個容器`K`和它分配的類型`A，B`，當`A < B`時，`K A < K B`
例如，`Option Int < Option Object`。因此，在`Option Object`中定義的方法也可以在`Option Int`中使用

考慮典型的多態類型 `Array!(T)`
請注意，這一次不是 `Array!(T, N)` 因為我們不關心元素的數量
現在，`Array!(T)` 類型具有稱為 `.push!` 和 `.pop!` 的方法，分別表示添加和刪除元素。這是類型: 

`Array.push!: Self(T).(T) => NoneType`
`Array.pop!: Self(T).() => T`

可以直觀地理解:

* `Array!(Object).push!(s)` is OK when `s: Str` (just upcast `Str` to `Object`)
* When `o: Object`, `Array!(Str).push!(o)` is NG
* `Array!(Object).pop!().into(Str)` is NG
* `Array!(Str).pop!().into(Object)` is OK

就類型系統而言，這是

* `(Self(Object).(Object) => NoneType) < (Self(Str).(Str) => NoneType)`
* `(Self(Str).() => Str) < (Self(Object).() => Object)`
方法

前者可能看起來很奇怪。即使是 `Str < Object`，包含關系在將其作為參數的函數中也是相反的
在類型論中，這種關系(`.push!` 的類型關系)稱為逆變，反之，`.pop!` 的類型關系稱為協變
換句話說，函數類型就其參數類型而言是逆變的，而就其返回類型而言是協變的
這聽起來很復雜，但正如我們之前看到的，如果將其應用于實際示例，這是一個合理的規則
如果您仍然不明白，請考慮以下內容

Erg 的設計原則之一是"大輸入類型，小輸出類型"。這正是函數可變性的情況
看上面的規則，輸入類型越大，整體類型越小
這是因為通用函數明顯比專用函數少
而且輸出類型越小，整體越小

這樣一來，上面的策略就相當于說"盡量減少函數的類型"

## 不變性

Erg 有另一個修改。它是不變的
這是對 `SharedCell! T!`等內置類型的修改。這意味著對于兩種類型 `T!, U!` 其中 `T! != U!`，在 `SharedCell! T!` 和 `SharedCell!意思是
這是因為`SharedCell！ T!` 是共享參考。有關詳細信息，請參閱 [共享參考](shared.md)

## 變異的泛型類型

通用類型變量可以指定其上限和下限

```python
|A <: T| K(A)
|B :> T| K(B)
```

在類型變量列表中，執行類型變量的__variant說明__。在上述變體規范中，類型變量"A"被聲明為"T"類型的任何子類，"B"類型被聲明為"T"類型的任何超類
在這種情況下，`T` 也稱為 `A` 的上部類型和 `B` 的下部類型

突變規范也可以重疊

```python
# U<A<T
{... | A<: T; A :> U}
```

這是使用變量規范的代碼示例: 

```python
show|S <: Show| s: S = log s

Nil T = Class(Impl = Phantom T)
Cons T = Class(Nil T or List T)
List T = Class {head = T; rest = Cons T}
List(T).
    push|U <: T|(self, x: U): List T = Self. new {head = x; rest = self}
    upcast(self, U :> T): List U = self
```

## 更改規范

`List T` 的例子很棘手，所以讓我們更詳細一點
要理解上面的代碼，你需要了解多態類型退化。[this section](./variance.md) 中詳細討論了方差，但現在我們需要三個事實: 

* 普通的多態類型，例如`List T`，與`T`是協變的(`List U > List T` when `U > T`)
* 函數 `T -> U` 對于參數類型 `T` 是逆變的(`(S -> U) < (T -> U)` when `S > T`)
* 函數 `T -> U` 與返回類型 `U` 是協變的(`(T -> U) > (T -> S)` 當 `U > S` 時)

例如，`List Int` 可以向上轉換為 `List Object`，而 `Obj -> Obj` 可以向上轉換為 `Int -> Obj`

現在讓我們考慮如果我們省略方法的變量說明會發生什么

```python
...
List T = Class {head = T; rest = Cons T}
List(T).
    # 如果 T > U，列表 T 可以被推入 U
    push|U|(self, x: U): List T = Self. new {head = x; rest = self}
    # List T 可以是 List U 如果 T < U
    upcast(self, U): List U = self
```

即使在這種情況下，Erg 編譯器也能很好地推斷 `U` 的上下類型
但是請注意，Erg 編譯器不理解方法的語義。編譯器只是根據變量和類型變量的使用方式機械地推斷和派生類型關系

正如評論中所寫，放在`List T`的`head`中的`U`類型是`T`的子類(`T: Int`，例如`Nat`)。也就是說，它被推斷為 `U <: T`。此約束將 `.push{U}` upcast `(List(T), U) -> List(T) 的參數類型更改為 (List(T), T) -> List(T)`(例如 disallow `列表(整數).push{對象}`)。但是請注意，`U <: T` 約束不會改變函數的類型包含。`(List(Int), Object) -> List(Int) to (List(Int), Int) -> List(Int)` 的事實并沒有改變，只是在 `.push` 方法中表示強制轉換無法執行
類似地，從 `List T` 到?? `List U` 的轉換可能會受到約束 `U :> T` 的約束，因此可以推斷出變體規范。此約束將 `.upcast(U)` 的返回類型更改為向上轉換 `List(T) -> List(T) 到 List(T) -> List(T)`(例如 `List(Object) .upcast(Int )`) 被禁止

現在讓我們看看如果我們允許這種向上轉換會發生什么
讓我們反轉變性名稱

```python
...
List T = Class {head = T; rest = Cons T}
List(T).
    push|U :> T|(self, x: U): List T = Self. new {head = x; rest = self}
    upcast(self, U :> T): List U = self
# 類型警告: `.push` 中的 `U` 不能接受除 `U == T` 之外的任何內容。將"U"替換為"T"
# 類型警告: `.upcast` 中的 `U` 不能接受除 `U == T` 之外的任何內容。將"U"替換為"T"
```

只有當 `U == T` 時，約束 `U <: T` 和修改規范`U :> T` 才滿足。所以這個稱號沒有多大意義
只有"向上轉換使得 `U == T`" = "向上轉換不會改變 `U` 的位置"實際上是允許的

## 附錄: 用戶定義類型的修改

默認情況下，用戶定義類型的突變是不可變的。但是，您也可以使用 `Inputs/Outputs` 標記Trait指定可變性
如果您指定 `Inputs(T)`，則類型相對于 `T` 是逆變的
如果您指定 `Outputs(T)`，則類型相對于 `T` 是協變的

```python
K T = Class(...)
assert not K(Str) <= K(Object)
assert not K(Str) >= K(Object)

InputStream T = Class ..., Impl := Inputs(T)
# 接受Objects的流也可以認為接受Strs
assert InputStream(Str) > InputStream(Object)

OutputStream T = Class ..., Impl := Outputs(T)
# 輸出Str的流也可以認為輸出Object
assert OutputStream(Str) < OutputStream(Object)
```
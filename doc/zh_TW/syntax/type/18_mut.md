# 可變類型

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/18_mut.md%26commit_hash%3D00682a94603fed2b531898200a79f2b4a64d5aae)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/18_mut.md&commit_hash=00682a94603fed2b531898200a79f2b4a64d5aae)

> __Warning__: 本節中的信息是舊的并且包含一些錯誤

默認情況下，Erg 中的所有類型都是不可變的，即它們的內部狀態無法更新
但是你當然也可以定義可變類型。變量類型用 `!` 聲明

```python
Person! = Class({name = Str; age = Nat!})
Person!.
    greet! ref! self = print! "Hello, my name is {self::name}. I am {self::age}."
    inc_age!ref!self = self::name.update!old -> old + 1
```

準確地說，基類型是可變類型或包含可變類型的復合類型的類型必須在類型名稱的末尾有一個"！"。沒有 `!` 的類型可以存在于同一個命名空間中，并被視為單獨的類型
在上面的例子中，`.age` 屬性是可變的，`.name` 屬性是不可變的。如果即使一個屬性是可變的，那么整個屬性也是可變的

可變類型可以定義重寫實例的過程方法，但具有過程方法并不一定使它們可變。例如數組類型`[T; N]` 實現了一個 `sample!` 隨機選擇一個元素的方法，但當然不會破壞性地修改數組

對可變對象的破壞性操作主要是通過 .update! 方法完成的。`.update!` 方法是一個高階過程，它通過應用函數 `f` 來更新 `self`

```python
i = !1
i.update! old -> old + 1
assert i == 2
```

`.set!` 方法只是丟棄舊內容并用新值替換它。.set!x = .update!_ -> x

```python
i = !1
i.set! 2
assert i == 2
```

`.freeze_map` 方法對不變的值進行操作

```python
a = [1, 2, 3].into [Nat; !3]
x = a.freeze_map a: [Nat; 3] -> a.iter().map(i -> i + 1).filter(i -> i % 2 == 0).collect(Array)
```

在多態不可變類型中，該類型的類型參數"T"被隱式假定為不可變

```python
# ImmutType < Type
KT: ImmutType = Class ...
K!T: Type = Class ...
```

在標準庫中，變量 `(...)!` 類型通常基于不可變 `(...)` 類型。但是，`T!` 和 `T` 類型沒有特殊的語言關系，并且不能這樣構造 [<sup id="f1">1</sup>](#1) 

請注意，有幾種類型的對象可變性
下面我們將回顧內置集合類型的不可變/可變語義

```python
# 數組類型
## 不可變類型
[T; N] # 不能執行可變操作
## 可變類型
[T; N] # 可以一一改變內容
[T; !N] # 可變長度，內容不可變但可以通過添加/刪除元素來修改
[!T; N] # 內容是不可變的對象，但是可以替換成不同的類型(實際上可以通過不改變類型來替換)
[!T; !N] # 類型和長度可以改變
[T; !N] # 內容和長度可以改變
[!T!; N] # 內容和類型可以改變
[!T!; !N] # 可以執行各種可變操作
```

當然，您不必全部記住和使用它們
對于可變數組類型，只需將 `!` 添加到您想要可變的部分，實際上是 `[T; N]`, `[T!; N]`，`[T; !N]`, ` [T!; !N]` 可以涵蓋大多數情況

這些數組類型是語法糖，實際類型是: 

```python
# actually 4 types
[T; N] = Array(T, N)
[T; !N] = Array!(T, !N)
[!T; N] = ArrayWithMutType!(!T, N)
[!T; !N] = ArrayWithMutTypeAndLength!(!T, !N)
[T!; !N] = Array!(T!, !N)
[!T!; N] = ArrayWithMutType!(!T!, N)
[!T!; !N] = ArrayWithMutTypeAndLength!(!T!, !N)
```

這就是能夠改變類型的意思

```python
a = [1, 2, 3].into [!Nat; 3]
a.map!(_ -> "a")
a: [!Str; 3]
```

其他集合類型也是如此

```python
# 元組類型
## 不可變類型
(T, U) # 元素個數不變，內容不能變
## 可變類型
(T!, U) # 元素個數不變，第一個元素可以改變
(T，U)！ # 元素個數不變，內容可以替換
...
```

```python
# 設置類型
## 不可變類型
{T; N} # 不可變元素個數，內容不能改變
## 可變類型
{T！; N} # 不可變元素個數，內容可以改變(一個一個)
{T; N}！ # 可變元素個數，內容不能改變
{T！; N}！ # 可變元素個數，內容可以改變
...
```

```python
# 字典類型
## 不可變類型
{K: V} # 長度不可變，內容不能改變
## 可變類型
{K:V!} # 恒定長度，值可以改變(一一)
{K: V}！ # 可變長度，內容不能改變，但可以通過添加或刪除元素來增加或刪除，內容類型也可以改變
...
```

```python
# 記錄類型
## 不可變類型
{x = Int; y = Str} # 內容不能改變
## 可變類型
{x = Int！; y = Str} # 可以改變x的值
{x = Int; y = Str}！ # 替換 {x = Int; 的任何實例 y = Str}
...
```

一個類型 `(...)` 簡單地變成了 `T! = (...)!` 當 `T = (...)` 被稱為簡單結構化類型。簡單的結構化類型也可以(語義上)說是沒有內部結構的類型
數組、元組、集合、字典和記錄類型都是非簡單的結構化類型，但 Int 和 Sieve 類型是

```python
# 篩子類型
## 枚舉
{1, 2, 3} # 1, 2, 3 之一，不可更改
{1、2、3}！ # 1、2、3，可以改
## 區間類型
1..12 # 1到12，不能改
1..12！ # 1-12中的任意一個，你可以改變
## 篩型(普通型)
{I: Int | I % 2 == 0} # 偶數類型，不可變
{I: Int | I % 2 == 0} # 偶數類型，可以改變
{I: Int | I % 2 == 0}！ # 與上面完全相同的類型，但上面的表示法是首選
```

從上面的解釋來看，可變類型不僅包括自身可變的，還包括內部類型可變的
諸如 `{x: Int!}` 和 `[Int!; 之類的類型3]` 是內部可變類型，其中內部的對象是可變的，而實例本身是不可變的

對于具有內部結構并在類型構造函數本身上具有 `!` 的類型 `K!(T, U)`，`*self` 可以更改整個對象。也可以進行局部更改
但是，希望盡可能保持本地更改權限，因此如果只能更改 `T`，最好使用 `K(T!, U)`
而對于沒有內部結構的類型‘T!’，這個實例只是一個可以交換的‘T’盒子。方法不能更改類型

---

<span id="1" style="font-size:x-small"><sup>1</sup> `T!` 和 `T` 類型沒有特殊的語言關系是有意的。這是一個設計。如果存在關系，例如命名空間中存在`T`/`T!`類型，則無法從其他模塊引入`T!`/`T`類型。此外，可變類型不是為不可變類型唯一定義的。給定定義 `T = (U, V)`，`T!` 的可能變量子類型是 `(U!, V)` 和 `(U, V!)`。[?](#f1)</span>

<p align='center'>
    <a href='./17'>上一頁</a> | <a href='./19_bound.md'>下一頁</a>
</p>
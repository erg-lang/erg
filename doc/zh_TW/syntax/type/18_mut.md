# 可變類型

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/18_mut.md%26commit_hash%3Dc6eb78a44de48735213413b2a28569fdc10466d0)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/18_mut.md&commit_hash=c6eb78a44de48735213413b2a28569fdc10466d0)

> __Warning__: 本節中的信息是舊的并且包含一些錯誤

默認情況下，Erg 中的所有類型都是不可變的，即它們的內部狀態無法更新
但是你當然也可以定義可變類型。變量類型用 `!` 聲明

```python
Person! = Class({name = Str; age = Nat!})
Person!.
    greet! ref! self = print! "Hello, my name is \{self::name}. I am \{self::age}."
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
x = a.freeze_map a: [Nat; 3] -> a.iter().map(i -> i + 1).filter(i -> i % 2 == 0).collect(List)
```

在多態不可變類型中，該類型的類型參數"T"被隱式假定為不可變

```python
# ImmutType < Type
KT: ImmutType = Class ...
K!T: Type = Class ...
```

在標準庫中，可變 `(...)!` 類型通常基于不可變 `(...)` 類型。但是，`T!` 和 `T` 類型沒有特殊的語言關系，并且不能這樣構造 [<sup id="f1">1</sup>](#1)

Types such as `{x: Int!}` and `[Int!; 3]` are internal mutable types where the object inside is mutable and the instance itself is not mutable.

## Cell! T

Mutable types are already available for `Int` and arrays, but how can we create mutable types for general immutable types? For example, in the case of `{x = Int; y = Int}`, corresponding mutable type is `{x = Int!; y = Int!}`, etc. But how did `Int!` made from `Int`?

Erg provides `Cell!` type for such cases.
This type is like a box for storing immutable types. This corresponds to what is called a reference (ref) in ML and other languages.

```python
IntOrStr = Inr or Str
IntOrStr! = Cell! IntOrStr
x = IntOrStr!.new 1
assert x is! 1 # `Int or Str` cannot compare with `Int` directly, so use `is!` (this compares object IDs) instead of `==`.
x.set! "a"
assert x is! "a"
```

An important property is that `Cell! T` is a subtype of `T`. Therefore, an object of type `Cell! T` can use all the methods of type `T`.

```python
# definition of `Int!`
Int! = Cell! Int
...
```

```python
i = !1
assert i == 1 # `i` is casted to `Int`
```

---

<span id="1" style="font-size:x-small"><sup>1</sup> `T!` 和 `T` 類型沒有特殊的語言關系是有意的。這是一個設計。如果存在關系，例如命名空間中存在`T`/`T!`類型，則無法從其他模塊引入`T!`/`T`類型。此外，可變類型不是為不可變類型唯一定義的。給定定義 `T = (U, V)`，`T!` 的可能變量子類型是 `(U!, V)` 和 `(U, V!)`。[?](#f1)</span>

<p align='center'>
    <a href='./17'>上一頁</a> | <a href='./19_bound.md'>下一頁</a>
</p>

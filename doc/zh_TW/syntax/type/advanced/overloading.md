# 重載

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/advanced/overloading.md%26commit_hash%3D51de3c9d5a9074241f55c043b9951b384836b258)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced/overloading.md&commit_hash=51de3c9d5a9074241f55c043b9951b384836b258)

Erg 不支持 __ad hoc 多態性__。 也就是說，函數和種類(重載)的多重定義是不可能的。 但是，您可以通過使用特征和補丁的組合來重現重載行為。
您可以使用特征而不是特征類，但隨后將涵蓋所有實現 `.add1` 的類型。

```python
Add1 = Trait {
    .add1: Self.() -> Self
}
IntAdd1 = Patch Int, Impl := Add1
IntAdd1.
    add1 self = self + 1
RatioAdd1 = Patch Ratio, Impl := Add1
RatioAdd1.
    add1 self = self + 1.0

add1|X <: Add1| x: X = x.add1()
assert add1(1) == 2
assert add1(1.0) == 2.0
```

這種接受一個類型的所有子類型的多態稱為__subtyping polymorphism__。

如果每種類型的過程完全相同，則可以編寫如下。 當行為從類到類(但返回類型相同)時，使用上述內容。
使用類型參數的多態稱為 __parametric polymorphism__。 參數多態性通常與子類型結合使用，如下所示，在這種情況下，它是參數和子類型多態性的組合。

```python
add1|T <: Int or Str| x: T = x + 1
assert add1(1) == 2
assert add1(1.0) == 2.0
```

此外，可以使用默認參數重現具有不同數量參數的類型的重載。

```python
C = Class {.x = Int; .y = Int}
C.
    new(x, y := 0) = Self::__new__ {.x; .y}

assert C.new(0, 0) == C.new(0)
```

Erg 的立場是，您不能定義行為完全不同的函數，例如根據參數的數量具有不同的類型，但如果行為不同，則應該以不同的方式命名。

綜上所述，Erg 禁止重載，采用子類型加參數多態，原因如下。

首先，重載函數分布在它們的定義中。 這使得在發生錯誤時很難報告錯誤的原因。
此外，導入子程序可能會改變已定義子程序的行為。

```python
{id; ...} = import "foo"
...
id x: Int = x
...
id x: Ratio = x
...
id "str" # 類型錯誤：沒有為 Str 實現 id
# 但是……但是……這個錯誤是從哪里來的？
```

其次，它與默認參數不兼容。 當具有默認參數的函數被重載時，會出現一個優先級的問題。

```python
f x: Int = ...
f(x: Int, y := 0) = ...

f(1) # 選擇哪個？
```

此外，它與聲明不兼容。
聲明 `f: Num -> Num` 不能指定它引用的定義。 這是因為 `Int -> Ratio` 和 `Ratio -> Int` 不包含在內。

```python
f: Num -> Num
f(x: Int): Ratio = ...
f(x: Ratio): Int = ...
```

并且語法不一致：Erg禁止變量重新賦值，但是重載的語法看起來像重新賦值。
也不能用匿名函數代替。

```python
# 同 `f = x -> body`
f x = body

# 一樣……什么？
f x: Int = x
f x: Ratio = x
```

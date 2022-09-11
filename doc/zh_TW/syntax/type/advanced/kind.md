# Kind

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/advanced/kind.md%26commit_hash%3Da9ea4eca75fe849e31f83570159f84b611892d7a)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced/kind.md&commit_hash=a9ea4eca75fe849e31f83570159f84b611892d7a)

一切都在 Erg 中輸入。類型本身也不例外。 __kind__ 表示"類型的類型"。例如，`Int` 屬于 `Type`，就像 `1` 屬于 `Int`。 `Type` 是最簡單的一種，__atomic kind__。在類型論符號中，`Type` 對應于 `*`。

在Kind的概念中，實際上重要的是一種或多種Kind(多項式Kind)。單項類型，例如`Option`，屬于它。一元Kind表示為 `Type -> Type` [<sup id="f1">1</sup>](#1)。諸如 `Array` 或 `Option` 之類的 __container__ 特別是一種以類型作為參數的多項式類型。
正如符號 `Type -> Type` 所表明的，`Option` 實際上是一個接收類型 `T` 并返回類型 `Option T` 的函數。但是，由于這個函數不是通常意義上的函數，所以通常稱為一元類。

注意`->`本身，它是一個匿名函數操作符，當它接收一個類型并返回一個類型時，也可以看作是一Kind型。

另請注意，不是原子Kind的Kind不是類型。正如 `-1` 是一個數字但 `-` 不是，`Option Int` 是一個類型但 `Option` 不是。 `Option` 等有時被稱為類型構造函數。

```python
assert not Option in Type
assert Option in Type -> Type
```

所以像下面這樣的代碼會報錯：
在 Erg 中，方法只能在原子類型中定義，并且名稱 `self` 不能在方法的第一個參數以外的任何地方使用。

```python
#K 是一元類型
K: Type -> Type
K T = Class...
K.
foo x = ... # OK，這就像是所謂的靜態方法
     bar self, x = ... # 類型錯誤: 無法為非類型對象定義方法
K(T).
    baz self, x = ... # OK
```

二進制或更高類型的示例是 `{T: U}`(: `(Type, Type) -> Type`), `(T, U, V)`(: `(Type, Type, Type) - > Type `), ... 等等。

還有一個零項類型`() -> Type`。 這有時等同于類型論中的原子類型，但在 Erg 中有所區別。 一個例子是`類`。

```python
Nil = Class()
```

## 收容類

多項類型之間也存在部分類型關系，或者更確切地說是部分類型關系。

```python
K T = ...
L = Inherit K
L<: K
```

也就是說，對于任何 `T`，如果 `L T <: K T`，則 `L <: K`，反之亦然。

```python
?T. L T <: K T <=> L <: K
```

## 高階Kind

還有一種高階Kind。 這是一種與高階函數相同的概念，一種自身接收一種類型。 `(Type -> Type) -> Type` 是一種更高的Kind。 讓我們定義一個屬于更高Kind的對象。

```python
IntContainerOf K: Type -> Type = K Int
assert IntContainerOf Option == Option Int
assert IntContainerOf Result == Result Int
assert IntContainerOf in (Type -> Type) -> Type
```

多項式類型的有界變量通常表示為 K, L, ...，其中 K 是 Kind 的 K

## 設置Kind

在類型論中，有記錄的概念。 這與 Erg 記錄 [<sup id="f2">2</sup>](#2) 幾乎相同。

```python
# 這是一條記錄，對應于類型論中所謂的記錄
{x = 1; y = 2}
```

當所有的記錄值都是類型時，它是一種類型，稱為記錄類型。

```python
assert {x = 1; y = 2} in {x = Int; y = Int}
```

記錄類型鍵入記錄。 一個好的猜測者可能認為應該有一個"記錄類型"來鍵入記錄類型。 實際上它是存在的。

```python
log Typeof {x = Int; y = Int} # {{x = Int; y = Int}}
```

像 `{{x = Int; 這樣的類型 y = Int}}` 是一種記錄類型。 這不是一個特殊的符號。 它只是一個枚舉類型，只有 `{x = Int; y = Int}` 作為一個元素。

```python
Point = {x = Int; y = Int}
Pointy = {Point}
```

記錄類型的一個重要屬性是，如果 `T: |T|` 和 `U <: T` 則 `U: |T|`。
從枚舉實際上是篩子類型的語法糖這一事實也可以看出這一點。

```python
# {c} == {X: T | X == c} 對于普通對象，但是不能為類型定義相等性，所以 |T| == {X | X <: T}
{Point} == {P | P <: Point}
```

類型約束中的 `U <: T` 實際上是 `U: |T|` 的語法糖。
作為此類類型的集合的種類通常稱為集合種類。 Setkind 也出現在迭代器模式中。

```python
Iterable T = Trait {
    .Iterator = {Iterator}
    .iter = (self: Self) -> Self.Iterator T
}
```

## 多項式類型的類型推斷

```python
Container K: Type -> Type, T: Type = Patch K(T, T)
Container (K).
    f self = ...
Option T: Type = Patch T or NoneType
Option(T).
    f self = ...
Fn T: Type = Patch T -> T
Fn(T).
    f self = ...
Fn2 T, U: Type = Patch T -> U
Fn2(T, U).
    f self = ...

(Int -> Int).f() # 選擇了哪一個?
```
在上面的示例中，方法 `f` 會選擇哪個補丁?
天真，似乎選擇了`Fn T`，但是`Fn2 T，U`也是可以的，`Option T`原樣包含`T`，所以任何類型都適用，`Container K，T`也匹配`->(Int, Int)`，即 `Container(`->`, Int)` 為 `Int -> Int`。因此，上述所有四個修復程序都是可能的選擇。

在這種情況下，根據以下優先標準選擇修復程序。

* 任何 `K(T)`(例如 `T or NoneType`)優先匹配 `Type -> Type` 而不是 `Type`。
* 任何 `K(T, U)`(例如 `T -> U`)優先匹配 `(Type, Type) -> Type` 而不是 `Type`。
* 類似的標準適用于種類 3 或更多。
* 選擇需要較少類型變量來替換的那個。例如，`Int -> Int` 是 `T -> T` 而不是 `K(T, T)`(替換類型變量：K, T)或 `T -> U`(替換類型變量：T, U )。(替換類型變量：T)優先匹配。
* 如果更換的次數也相同，則報錯為不可選擇。

---

<span id="1" style="font-size:x-small"><sup>1</sup> 在類型理論符號中，`*=>*` [?](#f1)</span>

<span id="2" style="font-size:x-small"><sup>2</sup> 可見性等細微差別。[?](#f2)</span>

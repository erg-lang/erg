# Typeof, classof

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/advanced/typeof.md%26commit_hash%3D44d7784aac3550ba97c8a1eaf20b9264b13d4134)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced/typeof.md&commit_hash=44d7784aac3550ba97c8a1eaf20b9264b13d4134)

`Typeof` 是一個可以窺探 Erg 類型推斷系統的函數，它的行為很復雜

```python
assert Typeof(1) == {I: Int | I == 1}
i: 1..3 or 5..10 = ...
assert Typeof(i) == {I: Int | (I >= 1 and I <= 3) or (I >= 5 and I <= 10)}

C = Class {i = Int}
I = C. new {i = 1}
assert Typeof(I) == {X: C | X == I}
J: C = ...
assert Typeof(J) == {i = Int}

assert {X: C | X == I} < C and C <= {i = Int}
```

`Typeof` 函數返回派生類型，而不是對象的類
因此，例如 `C = Class T` 類的`I: C`，`Typeof(I) == T`
值類沒有對應的記錄類型。為了解決這個問題，值類應該是具有 `__valueclass_tag__` 屬性的記錄類型
請注意，您不能訪問此屬性，也不能在用戶定義的類型上定義 `__valueclass_tag__` 屬性

```python
i: Int = ...
assert Typeof(i) == {__valueclass_tag__ = Phantom Int}
s: Str = ...
assert Typeof(s) == {__valueclass_tag__ = Phantom Str}
```

`Typeof` 僅輸出結構化類型。我解釋說結構化類型包括屬性類型、篩類型和(真正的)代數類型
這些是獨立的類型(存在推理優先級)，不會發生推理沖突
屬性類型和代數類型可以跨越多個類，而篩類型是單個類的子類型
Erg 盡可能將對象類型推斷為篩類型，如果不可能，則將篩基類擴展為結構化類型(見下文)

## 結構化的

所有類都可以轉換為派生類型。這稱為 __結構化__。類的結構化類型可以通過 `Structure` 函數獲得
如果一個類是用`C = Class T`定義的(所有類都以這種形式定義)，那么`Structure(C) == T`

```python
C = Class {i = Int}
assert Structure(C) == {i = Int}
D = Inherit C
assert Structure(D) == {i = Int}
Nat = Class {I: Int | I >= 0}
assert Structure(Nat) == {I: Int | I >= 0}
Option T = Class (T or NoneType)
assert Structure(Option Int) == Or(Int, NoneType)
assert Structure(Option) # 類型錯誤: 只能構造單態類型
# 你實際上不能用 __valueclass_tag__ 定義一條記錄，但在概念上
assert Structure(Int) == {__valueclass_tag__ = Phantom Int}
assert Structure(Str) == {__valueclass_tag__ = Phantom Str}
assert Structure((Nat, Nat)) == {__valueclass_tag__ = Phantom(Tuple(Nat, Nat))}
assert Structure(Nat -> Nat) == {__valueclass_tag__ = Phantom(Func(Nat, Nat))}
# 標記類也是帶有 __valueclass_tag__ 的記錄類型
M = Inherit Marker
assert Structure(M) == {__valueclass_tag__ = Phantom M}
D = Inherit(C and M)
assert Structure(D) == {i = Int; __valueclass_tag__ = Phantom M}
E = Inherit(Int and M)
assert Structure(E) == {__valueclass_tag__ = Phantom(And(Int, M))}
F = Inherit(E not M)
assert Structure(F) == {__valueclass_tag__ = Phantom Int}
```
# Typeof, classof

是可以窺視 Erg 的類型推理系統的函數，其舉動很複雜。


```erg
assert Typeof(1) == {I: Int | I == 1}
i: 1..3 or 5..10 = ...
assert Typeof(i) == {I: Int | (I >= 1 and I <= 3) or (I >= 5 and I <= 10)}

C = Class {i = Int}
I = C.new {i = 1}
assert Typeof(I) == {X: C | X == I}
J: C = ...
assert Typeof(J) == {i = Int}

assert {X: C | X == I} < C and C <= {i = Int}
```

函數返回的不是對象的類，而是結構類型。因此，對於<gtr=“6”/>類的實例<gtr=“7”/>，則為<gtr=“8”/>。關於值類，本來不存在對應的記錄類型。為了解決這個問題，值類是具有<gtr=“9”/>屬性的記錄型。此外，不能訪問該屬性，也不能在用戶定義類型中定義<gtr=“10”/>屬性。


```erg
i: Int = ...
assert Typeof(i) == {__valueclass_tag__ = Phantom Int}
s: Str = ...
assert Typeof(s) == {__valueclass_tag__ = Phantom Str}
```

用輸出的只是結構型。說明了結構型有屬性型、篩子型和（真的）代數演算型。這些是獨立的類型（存在推理的優先順序），不發生推理的重解。屬性型、代數運算型可能跨越多個類，而篩型是單一類的亞型。 Erg 盡可能地將對象的類型作為篩子類型進行推論，當不能進行推論時，將篩子類型的基類擴大到結構化（後述）的類型。

## 結構化

所有類都可以轉換為結構型。這被稱為。可以通過函數獲取類的結構化類型。如果用<gtr=“13”/>定義類（所有類都用這種形式定義），則<gtr=“14”/>。


```erg
C = Class {i = Int}
assert Structure(C) == {i = Int}
D = Inherit C
assert Structure(D) == {i = Int}
Nat = Class {I: Int | I >= 0}
assert Structure(Nat) == {I: Int | I >= 0}
Option T = Class (T or NoneType)
assert Structure(Option Int) == Or(Int, NoneType)
assert Structure(Option) # TypeError: only monomorphized types can be structurized
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
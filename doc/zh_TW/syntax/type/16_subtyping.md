# 部分定型

在 Erg 中，可以使用比較運算符和<gtr=“8”/>來確定類之間的包含關係。


```erg
Nat < Int
Int < Object
1.._ < Nat
{1, 2} > {1}
{=} > {x = Int}
{I: Int | I >= 1} < {I: Int | I >= 0}
```

請注意，它與運算符的含義不同。它聲明左側類是右側類型的子類型，並且僅在編譯時有意義。


```erg
C <: T # T: StructuralType
f|D <: E| ...

assert F < G
```

對於多相類型的子類型規範，例如，也可以指定<gtr=“11”/>。

## 結構類型，類類型關係

結構類型是用於實現結構定型的類型，如果結構相同，則將其視為相同的對象。


```erg
T = Structural {i = Int}
U = Structural {i = Int}

assert T == U
t: T = {i = 1}
assert t in T
assert t in U
```

相反，類是用於實現記名類型的類型，不能在結構上比較類型和實例。


```erg
C = Class {i = Int}
D = Class {i = Int}

assert C == D # TypeError: cannot compare classes
c = C.new {i = 1}
assert c in C
assert not c in D
```

## 子程序的局部類型

子程序的參數和返回值只採用單個類。也就是說，不能將結構型和trait作為函數的類型直接指定。必須使用子類型指定將其指定為“作為該類型子類型的單個類”。


```erg
# OK
f1 x, y: Int = x + y
# NG
f2 x, y: Add = x + y
# OK
# Aは何らかの具體的なクラス
f3<A <: Add> x, y: A = x + y
```

子程序的類型推論也遵循這個規則。當子程序中的變量中有未明示類型時，編譯器首先檢查該變量是否為某個類的實例，如果不是，則從作用域中的trait中尋找適合的變量。即使這樣也找不到的話，就成為編譯錯誤。這個錯誤可以通過使用結構型來消除，但是推論無名型有可能是程序員不想要的結果，所以設計成程序員明確地用來指定。

## 上傳類


```erg
i: Int
i as (Int or Str)
i as (1..10)
i as {I: Int | I >= 0}
```
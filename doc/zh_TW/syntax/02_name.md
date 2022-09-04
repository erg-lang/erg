# 變量

變量是代數的一種。 Erg 中的代數-有時也稱為變量（如果正確）-是指命名對象並使其可從代碼中的其他位置使用的功能。

變量定義如下。部分稱為變量名（或標識符），<gtr=“17”/>稱為賦值運算符，<gtr=“18”/>部分稱為賦值。


```erg
n = 1
```

以這種方式定義的隨後可用作表示整數對象<gtr=“20”/>的變量。此系統稱為賦值（或綁定）。我們剛才提到了<gtr=“21”/>是一個對象。我們將在後面討論對像是什麼，但我們現在應該將其賦值到賦值運算符（例如<gtr=“22”/>）的右側。

如果要指定變量的類型。類型是指對象所屬的集合，這也將在後面介紹。指定為自然數（<gtr=“24”/>）。


```erg
n: Nat = 1
```

請注意，與其他語言不同，多重賦值是不可能的。


```erg
# NG
l1 = l2 = [1, 2, 3] # SyntaxError: 多重代入はできません
# OK
l1 = [1, 2, 3]
l2 = l1.clone()
```

也不能對變量進行重新賦值。可以使用的功能，即保持可變狀態的功能將在後面討論。


```erg
i = 1
i = i + 1 # AssignError: cannot assign twice
```

你可以在內部範圍內定義具有相同名稱的變量，但它們只是放在上面，而不是破壞性地重寫值。如果返回到外部範圍，則值也將返回。請注意，這與 Python“語句”的作用域不同。這類功能通常稱為陰影。但是，與其他語言的陰影不同，你不能在同一範圍內進行陰影。


```erg
x = 0
# x = 1 # AssignError: cannot assign twice
if x.is_zero(), do:
    x = 1 # 外側のxとは同名の別物
    assert x == 1
assert x == 0
```

以下乍一看似乎可行，但還是不行。這不是技術限制，而是設計判斷。


```erg
x = 0
if x.is_zero(), do:
    x = x + 1 # NameError: cannot define variables refer to variables with the same name
    assert x == 1
assert x == 0
```

## 常數

常數也是代數的一種。如果標識符以大寫字母開頭，則將其視為常量。它被稱為常量，因為它一旦定義就不會改變。部分稱為常量名稱（或標識符）。其他與變量相同。


```erg
N = 0
if True, do:
    N = 1 # AssignError: constants cannot be shadowed
    pass()
```

常量在定義的範圍之後變得不變。我也不能陰影。由於該性質，常量可用於模式匹配。後面我們會討論模式匹配。

你可能希望將常量用於不變的值，如數學常量或有關外部資源的信息。除之外的對象通常是全部大寫字母（所有字符都是大寫的樣式）。


```erg
PI = 3.141592653589793
URL = "https://example.com"
CHOICES = ["a", "b", "c"]
```


```erg
PI = 3.141592653589793
match! x:
    PI => print! "π"
    other => print! "other"
```

當為<gtr=“28”/>時，上面的代碼輸出<gtr=“29”/>。如果將<gtr=“30”/>更改為其他數字，則輸出<gtr=“31”/>。

有些常量是不能賦值的。可變對像等等。可變對像是可以更改其內容的對象，如下所述。這是因為常量只能由常量表達式賦值。我們還將在後面討論常數表達式。


```erg
X = 1 # OK
X = !1 # TypeError: cannot define Int! object as a constant
```

## 刪除代數

可以使用函數刪除代數。所有依賴於代數（直接引用代數的值）的其他代數都將被刪除。


```erg
x = 1
y = 2
Z = 3
f a = x + a

assert f(2) == 3
Del x
Del y, Z

f(2) # NameError: f is not defined (deleted in line 6)
```

但是，只能刪除模塊中定義的代數。不能刪除內置常量，如<gtr=“34”/>。


```erg
Del True # TypeError: cannot delete built-in constants
Del print! # TypeError: cannot delete built-in variables
```

## Appendix：賦值等價性

注意，當時，不一定是<gtr=“36”/>。例如有<gtr=“37”/>。這是由 IEEE 754 規定的正式浮點數的規格。


```erg
x = Float.NaN
assert x != Float.NaN
assert x != x
```

其他，也存在原本就沒有定義等值關係的對象。


```erg
f = x -> x**2 + 2x + 1
g = x -> (x + 1)**2
f == g # TypeError: cannot compare function objects

C = Class {i: Int}
D = Class {i: Int}
C == D # TypeError: cannot compare class objects
```

嚴格地說，並不是將右邊值直接代入左邊的識別符。函數對象和類對象的情況下，對對象進行賦予變量名的信息等的“修飾”。但是結構型的情況不受此限制。


```erg
f x = x
print! f # <function f>
g x = x + 1
print! g # <function g>

C = Class {i: Int}
print! C # <class C>
```

<p align='center'>
    <a href='./01_literal.md'>Previous</a> | <a href='./03_declaration.md'>Next</a>
</p>
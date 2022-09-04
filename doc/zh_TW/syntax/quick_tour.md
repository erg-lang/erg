# Quick Tour

下面的文檔旨在讓初學者也能理解。對於已經掌握 Python，Rust，Haskell 等語言的人來說，這可能有點多餘。

因此，下面將概述性地介紹 Erg 的語法。沒有特別提到的部分可以認為和 Python 一樣。

## 變量，常量

變量定義為。與 Haskell 一樣，一旦定義變量，就無法重寫。但是，你可以在另一個範圍內進行陰影。


```erg
i = 0
if True:
    i = 1
assert i == 0
```

以大寫字母開頭的是常量。只有編譯時可以計算的內容才可以是常量。此外，常量在定義後的所有作用域中都是相同的。


```erg
PI = 3.141592653589793
match random.random!(0..10):
    PI:
        log "You get PI, it's a miracle!"
```

## 聲明

與 Python 不同，你只能先聲明變量類型。當然，聲明類型必須與實際賦值的對像類型兼容。


```erg
i: Int
i = 10
```

## 函數

你可以像 Haskell 一樣定義它。


```erg
fib 0 = 0
fib 1 = 1
fib n = fib(n - 1) + fib(n - 2)
```

可以按如下方式定義未命名函數。


```erg
i -> i + 1
assert [1, 2, 3].map(i -> i + 1).to_arr() == [2, 3, 4]
```

## 運算符

Erg 自己的運算符如下所示。

### 變量運算符（！）

就像 Ocaml 的。


```erg
i = !0
i.update! x -> x + 1
assert i == 1
```

## 過程

有副作用的子程序稱為過程，並帶有。


```erg
print! 1 # 1
```

## 類屬函數（多相關數）


```erg
id|T|(x: T): T = x
id(1): Int
id("a"): Str
```

## 記錄

你可以使用 ML 語言中的記錄（或 JS 中的對象文字）。


```erg
p = {x = 1; y = 2}
```

## 所有權

Erg 擁有可變對象（使用運算符可變的對象）的所有權，不能從多個位置重寫。


```erg
i = !0
j = i
assert j == 0
i # MoveError
```

相反，你可以從多個位置引用不變對象。

## 可見性

如果在變量的前面加上，則該變量將成為公共變量，並且可以被外部模塊引用。


```erg
# foo.er
.x = 1
y = 1
```


```erg
foo = import "foo"
assert foo.x == 1
foo.y # VisibilityError
```

## 模式匹配

### 變量模式


```erg
# basic assignment
i = 1
# with type
i: Int = 1
# function
fn x = x + 1
fn: Int -> Int = x -> x + 1
```

### 文字模式


```erg
# if `i` cannot be determined to be 1 at compile time, TypeError occurs.
# short hand of `_: {1} = i`
1 = i
# simple pattern matching
match x:
    1 -> "1"
    2 -> "2"
    _ -> "other"
# fibonacci function
fib 0 = 0
fib 1 = 1
fib n: Nat = fib n-1 + fib n-2
```

### 常數模式


```erg
PI = 3.141592653589793
E = 2.718281828459045
num = PI
name = match num:
    PI -> "pi"
    E -> "e"
    _ -> "unnamed"
```

### 銷毀（通配符）模式


```erg
_ = 1
_: Int = 1
right(_, r) = r
```

### 可變長度模式

與後述的元組/數組/記錄模式組合使用。


```erg
[i, ...j] = [1, 2, 3, 4]
assert j == [2, 3, 4]
first|T|(fst: T, ...rest: T) = fst
assert first(1, 2, 3) == 1
```

### 元組圖案


```erg
(i, j) = (1, 2)
((k, l), _) = ((1, 2), (3, 4))
# ネストしていないなら()を省略可能(1, 2は(1, 2)として扱われる)
m, n = 1, 2
```

### 數組模式


```erg
length [] = 0
length [_, ...rest] = 1 + length rest
```

#### 記錄模式


```erg
{sin; cos; tan; ...} = import "math"
{*} = import "math" # import all

person = {name = "John Smith"; age = 20}
age = match person:
    {name = "Alice"; _} -> 7
    {_; age} -> age
```

### 數據類模式


```erg
Point = Inherit {x = Int; y = Int}
p = Point::{x = 1; y = 2}
Point::{x; y} = p
```

## 內涵記載


```erg
odds = [i | i <- 1..100; i % 2 == 0]
```

## 類

Erg 不支持多級和多級繼承。

## trait

與 Rust 的trait類似，但更接近原意，可以合成和分離，屬性和方法是對等的。也不涉及實施。


```erg
XY = Trait {x = Int; y = Int}
Z = Trait {z = Int}
XYZ = XY and Z
Show = Trait {show: Self.() -> Str}

@Impl XYZ, Show
Point = Class {x = Int; y = Int; z = Int}
Point.
    ...
```

## 補丁

你可以為類和trait提供實現。

## 篩子型

可以在謂詞表達式中限制類型。


```erg
Nat = {I: Int | I >= 0}
```

## 包含值的參數化（從屬）


```erg
a: [Int; 3]
b: [Int; 4]
a + b: [Int; 7]
```
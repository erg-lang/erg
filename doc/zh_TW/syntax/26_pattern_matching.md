# 模式匹配，可辯駁性

## Erg 中可用的模式

### 變量模式


```erg
# basic assignment
i = 1
# with type
i: Int = 1
# with anonymous type
i: {1, 2, 3} = 2

# function
fn x = x + 1
# equals
fn x: Add(Int) = x + 1
# (anonymous) function
fn = x -> x + 1
fn: Int -> Int = x -> x + 1

# higher-order type
a: [Int; 4] = [0, 1, 2, 3]
# or
a: Array Int, 4 = [0, 1, 2, 3]
```

### 文字模式


```erg
# もし`i`がコンパイル時に1と判斷できない場合は、TypeErrorが発生する。
# `_: {1} = i`を省略したもの
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
cond = False
match! cond:
    True => print! "cond is True"
    _ => print! "cond is False"

PI = 3.141592653589793
E = 2.718281828459045
num = PI
name = match num:
    PI -> "pi"
    E -> "e"
    _ -> "unnamed"
```

### 篩子模式


```erg
# この２つは同じ
Array(T, N: {N | N >= 3})
Array(T, N | N >= 3)

f M, N | M >= 0, N >= 1 = ...
f(1, 0) # TypeError: N (2nd parameter) must be 1 or more
```

### 銷毀（通配符）模式


```erg
_ = 1
_: Int = 1
zero _ = 0
right(_, r) = r
```

### 可變長度模式

與下面介紹的元組/數組/記錄模式結合使用。


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

f(x, y) = ...
```

### 數組模式


```erg
[i, j] = [1, 2]
[[k, l], _] = [[1, 2], [3, 4]]

length [] = 0
length [_, ...rest] = 1 + length rest
```

#### 記錄模式


```erg
record = {i = 1; j = 2; k = 3}
{j; ...} = record # i, k will be freed

{sin; cos; tan; ...} = import "math"
{*} = import "math" # import all

person = {name = "John Smith"; age = 20}
age = match person:
    {name = "Alice"; _} -> 7
    {_; age} -> age

f {x: Int; y: Int} = ...
```

### 數據類模式


```erg
Point = Inherit {x = Int; y = Int}
p = Point::{x = 1; y = 2}
Point::{x; y} = p

Nil T = Class Impl := Phantom T
Cons T = Inherit {head = T; rest = List T}
List T = Enum Nil(T), Cons(T)
List T.
    first self =
        match self:
            Cons::{head; ...} -> x
            _ -> ...
    second self =
        match self:
            Cons::{rest=Cons::{head; ...}; ...} -> head
            _ -> ...
```

### 枚舉模式

※實際上是單純的列舉型


```erg
match x:
    i: {1, 2} -> "one or two: {i}"
    _ -> "other"
```

### 範圍模式

※實際上是單純的區間型


```erg
# 0 < i < 1
i: 0<..<1 = 0.5
# 1 < j <= 2
_: {[I, J] | I, J: 1<..2} = [1, 2]
# 1 <= i <= 5
match i
    i: 1..5 -> ...
```

### 不是模式的東西，不能被模式化的東西

模式可以是唯一的。在這一點上，模式匹配不同於常規條件分支。

條件指定不唯一。例如，如果確定數字是否為偶數，則<gtr=“14”/>是正統的，但也可以寫為<gtr=“15”/>。不唯一的格式不能明確表示是否正常工作，也不能明確表示是否等同於其他條件。

#### 設置

沒有佈景圖案。這是因為集合無法唯一地提取元素。可以用迭代器取出，但不保證順序。

<p align='center'>
    <a href='./25_object_system.md'>Previous</a> | <a href='./27_comprehension.md'>Next</a>
</p>
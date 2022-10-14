# 模式匹配，可反駁

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/26_pattern_matching.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/26_pattern_matching.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

## Erg 中可用的模式

### 變量模式

```python
# 基本任務
i = 1
# 有類型
i: Int = 1
# 匿名類型
i: {1, 2, 3} = 2

# 功能
fn x = x + 1
# 等于
fn x: Add(Int) = x + 1
# (匿名)函數
fn = x -> x + 1
fn: Int -> Int = x -> x + 1

# 高階類型
a: [Int; 4] = [0, 1, 2, 3]
# or
a: Array Int, 4 = [0, 1, 2, 3]
```

### 文字字面量

```python
# 如果在編譯時無法確定 `i` 為 1，則引發 TypeError
# 省略 `_: {1} = i`
1 = i

# 簡單的模式匹配
match x:
    1 -> "1"
    2 -> "2"
    _ -> "other"

# 斐波那契函數
fib0 = 0
fib1 = 1
fibn: Nat = fibn-1 + fibn-2
```

### 常量模式

```python
cond=False
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

### 篩子圖案

```python
# 這兩個是一樣的
Array(T, N: {N | N >= 3})
Array(T, N | N >= 3)

f M, N | M >= 0, N >= 1 = ...
f(1, 0) # 類型錯誤: N(第二個參數)必須為 1 或更多
```

### 丟棄(通配符)模式

```python
_ = 1
_: Int = 1
zero_ = 0
right(_, r) = r
```

### 可變長度模式

它與稍后描述的元組/數組/記錄模式結合使用

```python
[i,...j] = [1, 2, 3, 4]
assert j == [2, 3, 4]
first|T|(fst: T, ...rest: T) = fst
assert first(1, 2, 3) == 1
```

### 元組模式

```python
(i, j) = (1, 2)
((k, l), _) = ((1, 2), (3, 4))
# 如果不嵌套，() 可以省略(1, 2 被視為(1, 2))
m, n = 1, 2

f(x, y) = ...
```

### 數組模式

```python
[i, j] = [1, 2]
[[k, l], _] = [[1, 2], [3, 4]]

length[] = 0
length[_, ...rest] = 1 + lengthrest
```

#### record 模式

```python
record = {i = 1; j = 2; k = 3}
{j; ...} = record # i, k 將被釋放

{sin; cos; tan; ...} = import "math"
{*} = import "math" # import all

person = {name = "John Smith"; age = 20}
age = match person:
    {name = "Alice"; _} -> 7
    {_; age} -> age

f {x: Int; y: Int} = ...
```

### 數據類模式

```python
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

* 其實只是枚舉類型

```python
match x:
    i: {1, 2} -> "one or two: {i}"
    _ -> "other"
```

### Range 模式

* 實際上，它只是一個區間類型

```python
# 0 < i < 1
i: 0<..<1 = 0.5
# 1 < j <= 2
_: {[I, J] | I, J: 1<..2} = [1, 2]
# 1 <= i <= 5
match i
    i: 1..5 -> ...
```

### 不是模式的東西，不能被模式化的東西

模式是可以唯一指定的東西。在這方面，模式匹配不同于普通的條件分支

條件規格不是唯一的。例如，要檢查數字 `n` 是否為偶數，正統是 `n % 2 == 0`，但也可以寫成 `(n / 2).round() == n / 2`
非唯一形式無論是正常工作還是等效于另一個條件都不是微不足道的

#### Set

沒有固定的模式。因為集合沒有辦法唯一地檢索元素
您可以通過迭代器檢索它們，但不能保證順序

<p align='center'>
    <a href='./25_object_system.md'>上一頁</a> | <a href='./27_comprehension.md'>下一頁</a>
</p>
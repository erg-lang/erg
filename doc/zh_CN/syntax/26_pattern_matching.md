# 模式匹配，可反驳

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/26_pattern_matching.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/26_pattern_matching.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

## Erg 中可用的模式

### 变量模式

```python
# 基本任务
i = 1
# 有类型
i: Int = 1
# 匿名类型
i: {1, 2, 3} = 2

# 功能
fn x = x + 1
# 等于
fn x: Add(Int) = x + 1
# (匿名)函数
fn = x -> x + 1
fn: Int -> Int = x -> x + 1

# 高阶类型
a: [Int; 4] = [0, 1, 2, 3]
# or
a: Array Int, 4 = [0, 1, 2, 3]
```

### 文字字面量

```python
# 如果在编译时无法确定 `i` 为 1，则引发 TypeError
# 省略 `_: {1} = i`
1 = i

# 简单的模式匹配
match x:
    1 -> "1"
    2 -> "2"
    _ -> "other"

# 斐波那契函数
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

### 筛子图案

```python
# 这两个是一样的
Array(T, N: {N | N >= 3})
Array(T, N | N >= 3)

f M, N | M >= 0, N >= 1 = ...
f(1, 0) # 类型错误: N(第二个参数)必须为 1 或更多
```

### 丢弃(通配符)模式

```python
_ = 1
_: Int = 1
zero_ = 0
right(_, r) = r
```

### 可变长度模式

它与稍后描述的元组/数组/记录模式结合使用

```python
[i,...j] = [1, 2, 3, 4]
assert j == [2, 3, 4]
first|T|(fst: T, ...rest: T) = fst
assert first(1, 2, 3) == 1
```

### 元组模式

```python
(i, j) = (1, 2)
((k, l), _) = ((1, 2), (3, 4))
# 如果不嵌套，() 可以省略(1, 2 被视为(1, 2))
m, n = 1, 2

f(x, y) = ...
```

### 数组模式

```python
[i, j] = [1, 2]
[[k, l], _] = [[1, 2], [3, 4]]

length[] = 0
length[_, ...rest] = 1 + lengthrest
```

#### record 模式

```python
record = {i = 1; j = 2; k = 3}
{j; ...} = record # i, k 将被释放

{sin; cos; tan; ...} = import "math"
{*} = import "math" # import all

person = {name = "John Smith"; age = 20}
age = match person:
    {name = "Alice"; _} -> 7
    {_; age} -> age

f {x: Int; y: Int} = ...
```

### 数据类模式

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

### 枚举模式

* 其实只是枚举类型

```python
match x:
    i: {1, 2} -> "one or two: {i}"
    _ -> "other"
```

### Range 模式

* 实际上，它只是一个区间类型

```python
# 0 < i < 1
i: 0<..<1 = 0.5
# 1 < j <= 2
_: {[I, J] | I, J: 1<..2} = [1, 2]
# 1 <= i <= 5
match i
    i: 1..5 -> ...
```

### 不是模式的东西，不能被模式化的东西

模式是可以唯一指定的东西。在这方面，模式匹配不同于普通的条件分支

条件规格不是唯一的。例如，要检查数字 `n` 是否为偶数，正统是 `n % 2 == 0`，但也可以写成 `(n / 2).round() == n / 2`
非唯一形式无论是正常工作还是等效于另一个条件都不是微不足道的

#### Set

没有固定的模式。因为集合没有办法唯一地检索元素
您可以通过迭代器检索它们，但不能保证顺序

<p align='center'>
    <a href='./25_object_system.md'>上一页</a> | <a href='./27_comprehension.md'>下一页</a>
</p>
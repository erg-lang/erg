# パターンマッチ、論駁可能性


## Ergで使用可能なパターン

### 変数パターン

```python
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

### リテラルパターン

```python
# もし`i`がコンパイル時に1と判断できない場合は、TypeErrorが発生する。
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

### 定数パターン

```python
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

### 篩パターン

```python
# この２つは同じ
Array(T, N: {N | N >= 3})
Array(T, N | N >= 3)

f M, N | M >= 0, N >= 1 = ...
f(1, 0) # TypeError: N (2nd parameter) must be 1 or more
```

### 破棄(ワイルドカード)パターン

```python
_ = 1
_: Int = 1
zero _ = 0
right(_, r) = r
```

### 可変長パターン

後述するタプル/配列/レコードパターンと組み合わせて使います。

```python
[i, ...j] = [1, 2, 3, 4]
assert j == [2, 3, 4]
first|T|(fst: T, ...rest: T) = fst
assert first(1, 2, 3) == 1
```

### タプルパターン

```python
(i, j) = (1, 2)
((k, l), _) = ((1, 2), (3, 4))
# ネストしていないなら()を省略可能(1, 2は(1, 2)として扱われる)
m, n = 1, 2

f(x, y) = ...
```

### 配列パターン

```python
[i, j] = [1, 2]
[[k, l], _] = [[1, 2], [3, 4]]

length [] = 0
length [_, ...rest] = 1 + length rest
```

#### レコードパターン

```python
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

### データクラスパターン

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

### 列挙パターン

※実際には単なる列挙型

```python
match x:
    i: {1, 2} -> "one or two: {i}"
    _ -> "other"
```

### 範囲パターン

※実際には単なる区間型

```python
# 0 < i < 1
i: 0<..<1 = 0.5
# 1 < j <= 2
_: {[I, J] | I, J: 1<..2} = [1, 2]
# 1 <= i <= 5
match i
    i: 1..5 -> ...
```

### パターンではないもの、パターン化できないもの

パターンは一意に指定できるものです。この点においてパターンマッチは通常の条件分岐とは異なります。

条件の指定は一意ではありません。例えば、数`n`が偶数か判定する場合、`n % 2 == 0`とするのがオーソドックスですが、`(n / 2).round() == n / 2`とも書けます。
一意でない形式は、正しく作動するのか、別の条件と同等であるか自明ではありません。

#### セット

セットのパターンはありません。なぜなら、セットは要素を一意に取り出す方法がないからです。
イテレータで取り出すことはできますが、順番は保証されません。

<p align='center'>
    <a href='./25_object_system.md'>Previous</a> | <a href='./27_comprehension.md'>Next</a>
</p>

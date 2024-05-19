# 快速瀏覽

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/quick_tour.md%26commit_hash%3Dc6eb78a44de48735213413b2a28569fdc10466d0)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/quick_tour.md&commit_hash=c6eb78a44de48735213413b2a28569fdc10466d0)

`syntax` 下面的文檔是為了讓編程初學者也能理解而編寫的
對于已經掌握 Python、Rust、Haskell 等語言的人來說，可能有點啰嗦

所以，這里是 Erg 語法的概述
請認為未提及的部分與 Python 相同

## 基本計算

Erg 有一個嚴格的類型。但是, 由于類和Trait提供的靈活性, 類型會自動轉換為子類型(有關詳細信息，請參閱 [API](../API))

另外，不同的類型可以相互計算，只要類型是數值類型即可

```python
a = 1 # 1: Nat
b = a - 10 # -9: Int
c = b / 2 # -4.5: Float
d = c * 0 # -0.0: Float
e = f // 2 # 0: Nat
```

如果不想允許意外的類型擴展，可以在聲明時指定類型，以便在編譯時將其作為錯誤進行檢測。

```python
a = 1
b: Int = a / 2
# 錯誤信息
Error[#0047]: File <stdin>, line 1, in <module>
2│ b: Int = b / 2
   ^
類型錯誤: b的類型不匹配:
期待:  Int
但找到: Float
```
## 布爾類型
`True` 和 `False` 是 Boolean 類型的單例，但它們也可以轉換為 Int 類型
因此，如果它們是 Int 類型，則可以進行比較，但與其他類型比較會導致錯誤

```python
True == 1 # OK
False == 0 # OK
True == 1.0 # NG
False == 0.0 # NG
True == "a" # NG
```

## 變量，常量

變量用 `=` 定義。與 Haskell 一樣，變量一旦定義就不能更改。但是，它可以在另一個范圍內被遮蔽

```python
i = 0
if True:
    i = 1
assert i == 0
```

任何以大寫字母開頭的都是常數。只有可以在編譯時計算的東西才能是常量
此外，自定義以來，常量在所有范圍內都是相同的

```python
PI = 3.141592653589793
match random.random!(0..10):
    PI ->
        log "You get PI, it's a miracle!"
```

## 類型聲明

與 Python 不同的是，只能先聲明變量類型
當然，聲明的類型和實際分配的對象的類型必須兼容

```python
i: Int
i = 10
```

## 函數

你可以像在 Haskell 中一樣定義它

```python
fib 0 = 0
fib 1 = 1
fib n = fib(n - 1) + fib(n - 2)
```

匿名函數可以這樣定義:

```python
i -> i + 1
assert [1, 2, 3].map(i -> i + 1).to_arr() == [2, 3, 4]
```

## 運算符

特定于 Erg 的運算符是:

### 變異運算符 (!)

這就像 Ocaml 中的`ref`

```python
i = !0
i.update! x -> x + 1
assert i == 1
```

## 程序

具有副作用的子例程稱為過程，并標有`!`

您不能在函數中調用過程

```python
print! 1 # 1
```

## 泛型函數(多相關)

```python
id|T|(x: T): T = x
id(1): Int
id("a"): Str
```

## 記錄

您可以使用類似 ML 的語言中的記錄等價物(或 JS 中的對象字面量)

```python
p = {x = 1; y = 2}
assert p.x == 1
```

## 所有權

Ergs 由可變對象(使用 `!` 運算符突變的對象)擁有，并且不能從多個位置重寫

```python
i = !0
j = i
assert j == 0
i# 移動錯誤
```

另一方面，不可變對象可以從多個位置引用

## 可見性

使用 `.` 前綴變量使其成為公共變量并允許從外部模塊引用它

```python
# foo.er
.x = 1
y = 1
```

```python
foo = import "foo"
assert foo.x == 1
foo.y # 可見性錯誤
```

## 模式匹配

### 變量模式

```python
# 基本任務
i = 1
# with 類型
i: Int = 1
# 函數
fn x = x + 1
fn: Int -> Int = x -> x + 1
```

### 文字模式

```python
# 如果 `i` 在編譯時無法確定為 1，則發生 類型錯誤
# 簡寫: `_ {1} = i`
1 = i
# 簡單的模式匹配
match x:
    1 -> "1"
    2 -> "2"
    _ -> "other"
# 斐波那契函數
fib 0 = 0
fib 1 = 1
fib n: Nat = fibn-1 + fibn-2
```

### 常量模式

```python
PI = 3.141592653589793
E = 2.718281828459045
num = PI
name = match num:
    PI -> "pi"
    E -> "e"
    _ -> "unnamed"
```

### 丟棄(通配符)模式

```python
_ = 1
_: Int = 1
right(_, r) = r
```

### 可變長度模式

與稍后描述的元組/數組/記錄模式結合使用

```python
[i, *j] = [1, 2, 3, 4]
assert j == [2, 3, 4]
first|T|(fst: T, *rest: T) = fst
assert first(1, 2, 3) == 1
```

### 元組模式

```python
(i, j) = (1, 2)
((k, l), _) = ((1, 2), (3, 4))
# 如果不嵌套，() 可以省略(1, 2 被視為(1, 2))
m, n = 1, 2
```

### 數組模式

```python
length [] = 0
length [_, *rest] = 1 + length rest
```

#### 記錄模式

```python
{sin; cos; tan} = import "math"
{*} = import "math" # 全部導入

person = {name = "John Smith"; age = 20}
age = match person:
    {name = "Alice"; _} -> 7
    {_; age} -> age
```

### 數據類模式

```python
Point = Inherit {x = Int; y = Int}
p = Point::{x = 1; y = 2}
Point::{x; y} = p
```

## 理解(Comprehensions)

```python
odds = [i | i <- 1..100; i % 2 == 0]
```

## Class

Erg 不支持多重繼承

## Trait

它們類似于 Rust Trait，但在更字面意義上，允許組合和解耦，并將屬性和方法視為平等
此外，它不涉及實施

```python
XY = Trait {x = Int; y = Int}
Z = Trait {z = Int}
XYZ = XY and Z
Show = Trait {show: Self.() -> Str}

@Impl XYZ, Show
Point = Class {x = Int; y = Int; z = Int}
Point.
    ...
```

## 修補

您可以為類和Trait提供實現

## 篩子類型

謂詞表達式可以是類型限制的

```python
Nat = {I: Int | I >= 0}
```

## 帶值的參數類型(依賴類型)

```python
a: [Int; 3]
b: [Int; 4]
a + b: [Int; 7]
```

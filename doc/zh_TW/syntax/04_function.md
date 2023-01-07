# 函數

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/04_function.md%26commit_hash%3D96b113c47ec6ca7ad91a6b486d55758de00d557d)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/04_function.md&commit_hash=96b113c47ec6ca7ad91a6b486d55758de00d557d)

函數是一個塊，它接受一個"參數"，對其進行處理，并將其作為"返回值"返回。定義如下

```python
add x, y = x + y
```

```python
# 或者
add(x, y) = x + y
```

在函數名之后指定的名稱稱為參數
相反，傳遞給函數的對象稱為參數
函數 `add` 是一個以 `x` 和 `y` 作為參數并返回它們之和的函數，`x + y`
可以按如下方式調用(應用/調用)定義的函數

```python
add 1, 2
# or
add(1, 2)
```

## 冒號應用風格

函數像`f x, y, ...`一樣被調用，但是如果單行參數太多，可以使用`:`(冒號)來應用它們

```python,check_ignore
f some_long_name_variable_1 + some_long_name_variable_2, some_long_name_variable_3 * some_long_name_variable_4
```

```python,check_ignore
f some_long_name_variable_1 + some_long_name_variable_2:
    some_long_name_variable_3 * some_long_name_variable_4
```

```python
f:
    some_long_name_variable_1 + some_long_name_variable_2
    some_long_name_variable_3 * some_long_name_variable_4
```

以上三個代碼的含義相同。例如，這種風格在使用 `if` 函數時也很有用

```python
result = if Bool.sample!():
    do:
        log "True was chosen"
        1
    do:
        log "False was chosen"
        0
```

在 `:` 之后，除了注釋之外，不得編寫任何代碼，并且必須始終在新行上
此外，您不能在函數后立即使用 `:`。只有 `do`和`do!` 可以做到這一點

```python,compile_fail
# NG
f:
    x
    y
```

```python,checker_ignore
# Ok
f(
    x,
    y
)
```

## 關鍵字參數

如果使用大量參數定義函數，則存在以錯誤順序傳遞參數的危險
在這種情況下，使用關鍵字參數調用函數是安全的

```python
f x, y, z, w, v, u: Int = ...
```

上面定義的函數有很多參數，并且排列順序混亂。您不應該創建這樣的函數，但是在使用別人編寫的代碼時可能會遇到這樣的代碼。因此，我們使用關鍵字參數。如果使用關鍵字參數，則值會從名稱傳遞到正確的參數，即使它們的順序錯誤

```python
f u := 6, v := 5, w := 4, x := 1, y := 2, z := 3
```

## 定義默認參數

當某些參數大部分是固定的并且您希望能夠省略它們時，使用默認參數

默認參數由`:=`(walrus運算符)指定。如果未指定 `base`，則將 `math.E` 分配給 `base`

```python
math_log x: Ratio, base := math.E = ...

assert math_log(100, 10) == 2
assert math_log(100) == math_log(100, math.E)
```

請注意，不指定參數和指定`None`是有區別的

```python
p! x := 0 = print!
p!(2) # 2
p!() # 0
p!(None) # None
```

也可以與類型規范和模式一起使用

```python
math_log x, base: Ratio := math.E = ...
f [x, y] := [1, 2] = ...
```

但是，在默認參數中，不能調用過程(稍后描述)或分配可變對象

```python
f x := p! 1 = ... # NG
```

此外，剛剛定義的參數不能用作傳遞給默認參數的值

```python
f x := 1, y := x = ... # NG
```

## 可變長度參數

輸出其參數的日志(記錄)的 `log` 函數可以采用任意數量的參數

```python
log "你好", "世界", "！" # 你好 世界 ！
```

要定義這樣的函數，請將 `...` 添加到參數中。這樣，函數將參數作為可變長度數組接收

```python
f ...x =
    for x, i ->
        log i

# x == [1, 2, 3, 4, 5]
f 1, 2, 3, 4, 5
```

## 具有多種模式的函數定義

```python
fib n: Nat =
    match n:
        0 -> 0
        1 -> 1
        n -> fib(n - 1) + fib(n - 2)
```

像上面這樣的函數，其中 `match` 直接出現在定義下，可以重寫如下

```python
fib 0 = 0
fib 1 = 1
fib(n: Nat): Nat = fib(n - 1) + fib(n - 2)
```

注意一個函數定義有多個模式不是所謂的重載(multiple definition)； 一個函數只有一個定義。在上面的示例中，"n"必須與"0"或"1"屬于同一類型。此外，與 `match` 一樣，模式匹配是從上到下完成的

如果不同類的實例混合在一起，最后一個定義必須指定函數參數的類型為`Or`

```python
f "aa" = ...
f 1 = ...
# `f x = ... ` 無效
f x: Int or Str = ...
```

此外，像 `match` 一樣，它也必須是詳盡的

```python
fib 0 = 0
fib 1 = 1
# 模式錯誤: fib 參數的模式并不詳盡
```

但是，可以通過使用稍后描述的 [refinement type](./type/12_refinement.md) 顯式指定類型來使其詳盡無遺

```python
fib: 0..1 -> 0..1
fib 0 = 0
fib 1 = 1
# OK
```

## 遞歸函數

遞歸函數是在其定義中包含自身的函數

作為一個簡單的例子，讓我們定義一個執行階乘計算的函數`factorial`。階乘是"將所有小于或等于的正數相乘"的計算
5 的階乘是 `5*4*3*2*1 == 120`

```python
factorial 0 = 1
factorial 1 = 1
factorial(n: Nat): Nat = n * factorial(n - 1)
```

首先，從階乘的定義來看，0和1的階乘都是1
反過來，2的階乘是`2*1 == 2`，3的階乘是`3*2*1 == 6`，4的階乘是`4*3*2*1 == 24 `
如果我們仔細觀察，我們可以看到一個數 n 的階乘是前一個數 n-1 乘以 n 的階乘
將其放入代碼中，我們得到 `n * factorial(n - 1)`
由于 `factorial` 的定義包含自身，`factorial` 是一個遞歸函數

提醒一下，如果您不添加類型規范，則會這樣推斷

```python
factorial: |T <: Sub(Int, T) and Mul(Int, Int) and Eq(Int)| T -> Int
factorial 0 = 1
factorial 1 = 1
factorial n = n * factorial(n - 1)
```

但是，即使您可以推理，您也應該明確指定遞歸函數的類型。在上面的例子中，像"factorial(-1)"這樣的代碼可以工作，但是

```python
factorial(-1) == -1 * factorial(-2) == -1 * -2 * factorial(-3) == ...
```

并且這種計算不會停止。遞歸函數必須仔細定義值的范圍，否則您可能會陷入無限循環
所以類型規范也有助于避免接受意外的值

## High-order functions

高階函數是將函數作為參數或返回值的函數
例如，一個以函數為參數的高階函數可以寫成如下

```python
arg_f = i -> log i
higher_f(x: (Int -> NoneType)) = x 10
higher_f arg_f # 10
```

當然，也可以將返回值作為一個函數。

```python
add(x): (Int -> Int) = y -> x + y
add_ten = add(10) # y -> 10 + y
add_hundred = add(100) # y -> 100 + y
assert add_ten(1) == 11
assert add_hundred(1) == 101
```

通過這種方式將函數作為參數和返回值，可以用函數定義更靈活的表達式

## 編譯時函數

函數名以大寫字母開頭，表示編譯時函數。用戶定義的編譯時函數必須將所有參數作為常量，并且必須指定它們的類型
編譯時函數的功能有限。在編譯時函數中只能使用常量表達式，即只有一些運算符(例如求積、比較和類型構造操作)和編譯時函數。要傳遞的參數也必須是常量表達式
作為回報，優點是計算可以在編譯時完成

```python
Add(X, Y: Nat): Nat = X + Y
assert Add(1, 2) == 3

Factorial 0 = 1
Factorial(X: Nat): Nat = X * Factorial(X - 1)
assert Factorial(10) == 3628800

math = import "math"
Sin X = math.sin X # 常量錯誤: 此函數在編譯時不可計算
```

編譯時函數也用于多態類型定義

```python
Option T: Type = T or NoneType
Option: Type -> Type
```

## 附錄: 功能對比

Erg 沒有為函數定義 `==`。這是因為通常沒有函數的結構等價算法

```python
f = x: Int -> (x + 1)**2
g = x: Int -> x**2 + 2x + 1

assert f == g # 類型錯誤: 無法比較函數
```

盡管 `f` 和 `g` 總是返回相同的結果，但要做出這樣的決定是極其困難的。我們必須向編譯器教授代數
所以 Erg 完全放棄了函數比較，并且 `(x -> x) == (x -> x)` 也會導致編譯錯誤。這是與 Python 不同的規范，應該注意

```python
# Python，奇怪的例子
f = lambda x: x
assert f == f
assert (lambda x: x) ! = (lambda x: x)
```

## Appendix2: ()-completion

```python
f x: Object = ...
# 將完成到
f(x: Object) = ...

f a
# 將完成到
f(a)

f a, b # 類型錯誤: f() 接受 1 個位置參數，但給出了 2 個
f(a, b) # # 類型錯誤: f() 接受 1 個位置參數，但給出了 2 個
f((a, b)) # OK
```

函數類型`T -> U`實際上是`(T,) -> U`的語法糖

<p align='center'>
    <a href='./03_declaration.md'>上一頁</a> | <a href='./05_builtin_funcs.md'>下一頁</a>
</p>

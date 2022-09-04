# 函數

函數是一個塊，它接受參數並對其進行處理，然後將其作為返回值返回。定義如下。


```erg
add x, y = x + y
# or
add(x, y) = x + y
```

在定義函數時指定的參數通常稱為偽參數（parameter）。相反，函數調用過程中傳遞的參數稱為實際參數（argument）。是接受<gtr=“31”/>和<gtr=“32”/>作為假參數，然後返回<gtr=“33”/>的函數。你可以按如下方式調用（應用）定義的函數。


```erg
add 1, 2
# or
add(1, 2)
```

## 冒號樣式

函數的調用方式如下：，但如果實際參數太多，一行太長，則可以使用<gtr=“35”/>（冒號）來應用。


```erg
f some_long_name_variable_1 + some_long_name_variable_2, some_long_name_variable_3 * some_long_name_variable_4
```


```erg
f some_long_name_variable_1 + some_long_name_variable_2:
    some_long_name_variable_3 * some_long_name_variable_4
```


```erg
f:
    some_long_name_variable_1 + some_long_name_variable_2
    some_long_name_variable_3 * some_long_name_variable_4
```

上面三個代碼都是同一個意思。此樣式在使用函數時也很有用。


```erg
result = if Bool.sample!():
    do:
        log "True was chosen"
        1
    do:
        log "False was chosen"
        0
```

在之後，不能寫註釋以外的代碼，必須換行。

## 關鍵字參數（Keyword Arguments）

如果定義了具有大量參數的函數，則可能會導致傳遞參數的順序錯誤。在這種情況下，使用關鍵字參數進行調用是安全的。


```erg
f x, y, z, w, v, u: Int = ...
```

上面定義的函數有很多參數，並且排列很難懂。我們不應該做這樣的函數，但在使用別人寫的代碼時可能會碰到這樣的代碼。因此，我們使用關鍵字參數。關鍵字參數的名稱優先於順序，因此即使順序不正確，也會將值從名稱傳遞到正確的參數。


```erg
f u: 6, v: 5, w: 4, x: 1, y: 2, z: 3
```

請注意，如果在關鍵字參數和之後立即換行，將被視為冒號應用樣式。


```erg
# means `f(x: y)`
f x: y

# means `f(x, y)`
f x:
    y
```

## 默認參數（Default parameters）

如果一個參數在大多數情況下是固定的，並且你想要省略它，則可以使用默認參數。

缺省參數由（or-assign operator）指定。如果未指定<gtr=“40”/>，則將<gtr=“41”/>賦給<gtr=“42”/>。


```erg
math_log x: Ratio, base := math.E = ...

assert math_log(100, 10) == 2
assert math_log(100) == math_log(100, math.E)
```

請注意，不指定參數和賦值是有區別的。


```erg
p! x := 0 = print! x
p!(2) # 2
p!() # 0
p!(None) # None
```

也可以與類型和模式一起使用。


```erg
math_log x, base: Ratio := math.E = ...
f [x, y] := [1, 2] = ...
```

但是，在缺省參數中，不能調用以下過程或賦值可變對象。


```erg
f x := p! 1 = ... # NG
```

此外，不能將剛定義的參數用作傳遞給缺省參數的值。


```erg
f x := 1, y := x = ... # NG
```

## 可變長度參數

函數將參數作為日誌輸出，可以接收任意數量的參數。


```erg
log "Hello", "World", "!" # Hello World !
```

如果要定義這樣的函數，請將作為參數。這樣，參數就可以作為可變長度數組接收。


```erg
f x: ...Int =
    for x, i ->
        log i

# x == [1, 2, 3, 4, 5]
f 1, 2, 3, 4, 5
```

## 多模式函數定義


```erg
fib n: Nat =
    match n:
        0 -> 0
        1 -> 1
        n -> fib(n - 1) + fib(n - 2)
```

如果函數的定義正下方出現，如上面所示，則可以重寫如下所示。


```erg
fib 0 = 0
fib 1 = 1
fib(n: Nat): Nat = fib(n - 1) + fib(n - 2)
```

請注意，多模式函數定義不是所謂的過載（多重定義）。一個函數始終只有一個類型。在上面的示例中，必須與<gtr=“48”/>和<gtr=“49”/>具有相同的類型。此外，與<gtr=“50”/>相同，模式匹配從上到下依次進行。

如果存在不同類的混合實例，則必須在最後一個定義中指明函數參數類型為 Or。


```erg
f "aa" = ...
f 1 = ...
# `f x = ...` is invalid
f x: Int or Str = ...
```

它還必須具有包容性，如。


```erg
fib 0 = 0
fib 1 = 1
# PatternError: pattern of fib's parameter is not exhaustive
```

但是，即使在上述情況下，也可以使用下面的顯式指定類型來獲得全面性。


```erg
fib: 0..1 -> 0..1
fib 0 = 0
fib 1 = 1
# OK
```

## 遞歸函數

遞歸函數是定義中包含自身的函數。

作為一個簡單的例子，我們嘗試定義函數來計算階乘。階乘是“乘以所有小於或等於的正數”的計算。 5 的階乘為。


```erg
factorial 0 = 1
factorial 1 = 1
factorial(n: Nat): Nat = n * factorial(n - 1)
```

首先從階乘定義開始，0 和 1 的階乘都是 1. 按順序計算，2 的階乘為，3 的階乘為，4 的階乘為。如果你仔細觀察這裡，你會發現一個數字 n 的階乘是它前面的數字 n-1 的階乘乘以 n。如果你將其放入代碼中，則會得到。 <gtr=“60”/>是遞歸函數，因為<gtr=“59”/>的定義包含它自己。

注意，如果未指定類型，則會這樣推斷。


```erg
factorial: |T <: Sub(Int, T) and Mul(Int, Int) and Eq(Int)| T -> Int
factorial 0 = 1
factorial 1 = 1
factorial n = n * factorial(n - 1)
```

但是，即使可以推理，也應該明確指定遞歸函數的類型。在上面的示例中，像這樣的代碼是有效的，


```erg
factorial(-1) == -1 * factorial(-2) == -1 * -2 * factorial(-3) == ...
```

，此計算不會停止。如果不仔細定義值的範圍，遞歸函數可能會陷入無限循環。類型還有助於防止接受不想要的值。

## 編譯時函數

如果函數名以大寫字母開頭，則該函數為編譯時函數。所有用戶定義的編譯時函數的參數都必須是常量，並且必須顯式。編譯函數能做的事情是有限的。在編譯時函數中只能使用常量表達式，即某些運算符（四則運算，比較運算，類型構建運算等）和編譯時函數。賦值的參數也必須是常量表達式。相反，計算可以在編譯時進行。


```erg
Add(X, Y: Nat): Nat = X + Y
assert Add(1, 2) == 3

Factorial 0 = 1
Factorial(X: Nat): Nat = X * Factorial(X - 1)
assert Factorial(10) == 3628800

math = import "math"
Sin X = math.sin X # ConstantError: this function is not computable at compile time
```

編譯時函數通常用於多相類型定義等。


```erg
Option T: Type = T or NoneType
Option: Type -> Type
```

## Appendix：比較函數

Erg 沒有為函數定義。那是因為函數的結構等價性判定算法一般不存在。


```erg
f = x: Int -> (x + 1)**2
g = x: Int -> x**2 + 2x + 1

assert f == g # TypeError: cannot compare functions
```

和<gtr=“64”/>總是返回相同的結果，但這是非常困難的。我們得把代數學灌輸給編譯器。因此，Erg 放棄了整個函數比較，<gtr=“65”/>也會導致編譯錯誤。這是與 Python 不同的規格，需要注意。


```python
# Python, weird example
f = lambda x: x
assert f == f
assert (lambda x: x) != (lambda x: x)
```

## Appendix2：完成（）


```erg
f x: Object = ...
# will be completed to
f(x: Object) = ...

f a
# will be completed to
f(a)

f a, b # TypeError: f() takes 1 positional argument but 2 were given
f(a, b) # TypeError: f() takes 1 positional argument but 2 were given
f((a, b)) # OK
```

函數類型實際上是<gtr=“67”/>的语法糖。

<p align='center'>
    <a href='./03_declaration.md'>Previous</a> | <a href='./05_builtin_funcs.md'>Next</a>
</p>
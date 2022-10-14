# rank-2 多態性

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/advanced/_rank2type.md%26commit_hash%3Da9ea4eca75fe849e31f83570159f84b611892d7a)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced/_rank2type.md&commit_hash=a9ea4eca75fe849e31f83570159f84b611892d7a)

> __Warning__: 本文檔已過時，一般包含錯誤

Erg 允許您定義接受各種類型的函數，例如 `id|T|(x: T): T = x`，即多相關
那么，我們可以定義一個接受多相關的函數嗎?
比如這樣的函數(注意這個定義是錯誤的): 

```python
# 我想要 tuple_map(i -> i * 2, (1, "a")) == (2, "aa")
tuple_map|T|(f: T -> T, tup: (Int, Str)): (Int, Str) = (f(tup.0), f(tup.1))
```

注意 `1` 和 `"a"` 有不同的類型，所以匿名函數一次不是單態的。需要單相兩次
這樣的函數不能在我們目前討論的類型范圍內定義。這是因為類型變量沒有范圍的概念
讓我們暫時離開類型，看看值級別的范圍概念

```python
arr = [1, 2, 3]
arr.map i -> i + 1
```

上面代碼中的 `arr` 和 `i` 是不同作用域的變量。因此，每個壽命都是不同的(`i` 更短)

到目前為止，所有類型變量的類型都具有相同的生命周期。換句話說，‘T’、‘X’和‘Y’必須同時確定，之后保持不變
反之，如果我們可以將 `T` 視為"內部作用域"中的類型變量，我們可以組成一個 `tuple_map` 函數。__Rank 2 type__ 就是為此目的而準備的

```python
# tuple_map: ((|T: Type| T -> T), (Int, Str)) -> (Int, Str)
tuple_map f: (|T: Type| T -> T), tup: (Int, Str) = (f(tup.0), f(tup.1))
assert tuple_map(i -> i * 2, (1, "a")) == (2, "aa")
```

`{(type) | 形式的類型 (類型變量列表)}` 被稱為通用類型(詳見[通用類型](../15_quantified.md))
目前我們看到的`id`函數是一個典型的通用函數=多相關函數

```python
id x = x
id: |T: Type| T -> T
```

通用類型與函數類型構造函數`->`的關聯有特殊的規則，根據關聯的方式，類型的語義是完全不同的

用簡單的單參數函數來考慮這一點

```python
f1: (T -> T) -> 整數 | T # 接受任何函數并返回 Int 的函數
f2: (|T: Type| T -> T) -> Int # 接收多相關并返回 Int 的函數
f3: Int -> (|T: Type| T -> T) # 一個函數，接受一個 Int 并返回一個封閉的通用函數
f4: |T: Type|(Int -> (T -> T)) # 同上(首選)
```

`f1` 和 `f2` 不同，而 `f3` 和 `f4` 相同，這似乎很奇怪。讓我們實際構造一個這種類型的函數

```python
# id: |T: Type| T -> T
id x = x
# same type as `f1`
take_univq_f_and_return_i(_: (|T: Type| T -> T), i: Int): Int = i
# same type as `f2`
take_arbit_f_and_return_i|T: Type|(_: T -> T, i: Int): Int = i
# same type as `f3`
take_i_and_return_univq_f(_: Int): (|T: Type| T -> T) = id
# same type as `f4`
take_i_and_return_arbit_f|T: Type|(_: Int): (T -> T) = id
```

After applying it, you will notice the difference.

```python
_ = take_univq_f_and_return_i(x -> x, 1) # OK
_ = take_univq_f_and_return_i(x: Int -> x, 1) #NG
_ = take_univq_f_and_return_i(x: Str -> x, 1) # NG
_ = take_arbit_f_and_return_i(x -> x, 1) # OK
_ = take_arbit_f_and_return_i(x: Int -> x, 1) # OK
_ = take_arbit_f_anf_return_i(x: Str -> x, 1) # OK

f: |T| T -> T = take_i_and_return_univq_f(1)
g: |T| T -> T = take_i_and_return_arbit_f(1)
assert f == g
f2: Int -> Int = take_i_and_return_univq_f|Int|(1)
g2: Int -> Int = take_i_and_return_arbit_f|Int|(1)
assert f2 == g2
```

開放的多相關函數類型具體稱為 __任意函數類型__。任意函數類型有無數種可能性: `Int -> Int`、`Str -> Str`、`Bool -> Bool`、`|T: Type| T -> T`, ... 是
另一方面，只有一個封閉的(返回與參數相同類型的對象)多態類型`|T: Type| T -> T`。這種類型被專門稱為 __多態函數類型__
也就是說，`f1`可以通過`x: Int -> x+1`、`x: Bool -> not x`、`x -> x`等=`f1`是一個多相關數是的，但是您只能將 `x -> x` 等傳遞給 `f2` = `f2` 不是 __多元相關__
但是像`f2`這樣的函數類型明顯不同于普通類型，我們需要新的概念來處理它們。那是類型的"等級"

關于rank的定義，沒有量化的類型，如`Int`、`Str`、`Bool`、`T`、`Int -> Int`、`Option Int`等，都被視為"rank" 0"

```python
# K 是多項式類型，例如 Option
R0 = (Int or Str or Bool or ...) or (R0 -> R0) or K(R0)
```

接下來，具有一階全稱的類型，例如`|T| T -> T`，或者在返回值類型中包含它們的類型是"rank 1"
此外，具有二階全稱量化的類型(具有 rank 1 類型作為參數的類型，例如 `(|T| T -> T) -> Int`)或將它們包含在返回類型中的類型稱為"rank 2 "
重復上述以定義"Rank N"類型。此外，秩-N 類型包括秩為N 或更少的所有類型。因此，混合等級的類型與其中最高的等級相同

```python
R1 = (|...| R0) or (R0 -> R1) or K(R1) or R0
R2 = (|...| R1) or (R1 -> R2) or K(R2) or R1
...
Rn = (|...| Rn-1) or (Rn-1 -> Rn) or K(Rn) or Rn-1
```

讓我們看看例子: 

```python
    (|T: Type| T -> T) -> (|U: Type| U -> U)
=> R1 -> R1
=> R1 -> R2
=> R2

Option(|T: Type| T -> T)
=> Option(R1)
=> K(R1)
=> R1
```

根據定義，`tuple_map` 是 rank-2 類型

```python
tuple_map:
    ((|T: Type| T -> T), (Int, Str)) -> (Int, Str)
=> (R1, R0) -> R0
=> R1 -> R2
=> R2
```

Erg 最多可以處理 rank 2 的類型(因為 rank N 類型包括所有 rank N 或更少的類型，確切地說，所有 Erg 類型都是 rank 2 類型)。試圖構造更多類型的函數是錯誤的
例如，所有處理多相關的函數都需要指定其他參數類型。而且，這樣的功能是不可配置的

```python
# 這是一個 rank-3 類型的函數
# |X, Y: Type|((|T: Type| T -> T), (X, Y)) -> (X, Y)
generic_tuple_map|X, Y: Type| f: (|T: Type| T -> T), tup: (X, Y) = (f(tup.0), f(tup.1))
```

眾所周知，具有 3 級或更高等級的類型在理論上無法通過類型推斷來確定。然而，大多數實際需求可以被等級 2 類型覆蓋。
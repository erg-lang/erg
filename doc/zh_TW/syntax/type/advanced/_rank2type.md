# rank-2 多相

> ：此文檔的信息較舊，通常包含錯誤。

在 Erg 中，像等可以接受各種類型的函數，即可以定義多相關數。那麼，能接受多相關數的函數能被定義嗎？例如，這樣的函數（請注意，此定義包含錯誤）。


```erg
# tuple_map(i -> i * 2, (1, "a")) == (2, "aa")我要你成為
tuple_map|T|(f: T -> T, tup: (Int, Str)): (Int, Str) = (f(tup.0), f(tup.1))
```

請注意，由於和的類型不同，因此無名函數並不是單相化一次就結束的。需要進行兩次單相化。在至今為止說明的型的範疇中，無法對這樣的函數進行定義。因為型變量中沒有範圍的概念。在此暫時離開類型，確認值水平上的範圍概念。


```erg
arr = [1, 2, 3]
arr.map i -> i + 1
```

上述代碼中的和<gtr=“18”/>是不同作用域的變量。因此，它們的生存期是不同的（<gtr=“19”/>更短）。

到目前為止的類型，所有的類型變量的生存期都是相同的。也就是說，，<gtr=“21”/>，<gtr=“22”/>同時被確定，以後必須不變。反過來說，如果可以將<gtr=“23”/>看作“內側範圍”中的類型變量，則可以構成<gtr=“24”/>函數。為此準備了<gtr=“25”/>。


```erg
# tuple_map: ((|T: Type| T -> T), (Int, Str)) -> (Int, Str)
tuple_map f: (|T: Type| T -> T), tup: (Int, Str) = (f(tup.0), f(tup.1))
assert tuple_map(i -> i * 2, (1, "a")) == (2, "aa")
```

形式的類型稱為全稱類型（詳細情況請參照<gtr=“28”/>）。至今所見的函數是典型的全稱函數 = 多相關數。


```erg
id x = x
id: |T: Type| T -> T
```

全稱型與函數型構建子之間具有特殊的結合規則，根據結合方法的不同，類型的意義完全不同。

對此，使用單純的 1 自變量函數進行考慮。


```erg
f1: (T -> T) -> Int | T # 接受任何函數並返回 Int 的函數
f2: (|T: Type| T -> T) -> Int # 接收多相關並返回 Int 的函數
f3: Int -> (|T: Type| T -> T) # 一個函數，接受一個 Int 並返回一個封閉的通用函數
f4: |T: Type|(Int -> (T -> T)) # 同上（首選）
```

和<gtr=“31”/>相同，而<gtr=“32”/>和<gtr=“33”/>卻不同，這似乎很奇怪。實際上試著構成這種類型的函數。


```erg
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

應用之後，就會發現其中的差異。


```erg
_ = take_univq_f_and_return_i(x -> x, 1) # OK
_ = take_univq_f_and_return_i(x: Int -> x, 1) # NG
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

開放的多相關數型特別稱為。任意函數類型有無限個可能性，如，<gtr=“35”/>，<gtr=“36”/>，<gtr=“37”/>，...等。與此相對，關閉的多相關數類型（返回與參數類型相同的對象）只有<gtr=“38”/>一種。這種類型特別稱為。換句話說，可以向<gtr=“39”/>傳遞<gtr=“40”/>、<gtr=“41”/>、<gtr=“42”/>等的 =<gtr=“43”/>是多相關數，但是可以向<gtr=“44”/>傳遞的只有<gtr=“45”/>等 =<gtr=“46”/>是多相關數<gtr=“50”/>。但是，像<gtr=“47”/>這樣的函數的類型明顯與通常的類型不同，需要能夠很好地處理這些的新概念。這就是套路的“檔次”。

關於等級的定義，首先，未量化的類型，即，<gtr=“52”/>，<gtr=“53”/>，<gtr=“54”/>，<gtr=“55”/>，<gtr=“56”/>等被認為是“等級 0”。


```erg
# KはOptionなどの多項カインド
R0 = (Int or Str or Bool or ...) or (R0 -> R0) or K(R0)
```

其次，將等進行一階全稱量化的類型，或者將其包含在返回值類型中的類型作為“等級 1”。此外，將進行二階全稱量化的類型（以等等級 1 類型為自變量的類型），或將其包含在返回值類型中的類型設為“等級 2”。重複上述操作，定義“秩 N”型。另外，等級 N 型包含 N 以下等級的所有類型。因此，多個等級混合的類型的等級與其中最高的等級相同。


```erg
R1 = (|...| R0) or (R0 -> R1) or K(R1) or R0
R2 = (|...| R1) or (R1 -> R2) or K(R2) or R1
...
Rn = (|...| Rn-1) or (Rn-1 -> Rn) or K(Rn) or Rn-1
```

讓我們來看幾個例子。


```erg
    (|T: Type| T -> T) -> (|U: Type| U -> U)
=>  R1 -> R1
=>  R1 -> R2
=>  R2

Option(|T: Type| T -> T)
=>  Option(R1)
=>  K(R1)
=>  R1
```

根據定義，是等級 2 型。


```erg
tuple_map:
    ((|T: Type| T -> T), (Int, Str)) -> (Int, Str)
=>  (R1, R0) -> R0
=>  R1 -> R2
=>  R2
```

在 Erg 中，可以處理到等級 2 為止的類型（等級 N 型包含 N 以下等級的所有類型，因此正確地說 Erg 的類型都是等級 2 型）。如果試圖配置更多類型的函數，則會出現錯誤。例如，將多相關數作為多相關數處理的函數都需要指定其他自變量的類型。另外，不能構成這樣的函數。


```erg
# this is a rank-3 type function
# |X, Y: Type|((|T: Type| T -> T), (X, Y)) -> (X, Y)
generic_tuple_map|X, Y: Type| f: (|T: Type| T -> T), tup: (X, Y) = (f(tup.0), f(tup.1))
```

等級 3 以上的類型在理論上不能決定類型推論的事實已知，類型指定破壞了可以省略的 Erg 的性質，因此被排除。儘管如此，實用需求 2 級基本可以覆蓋。
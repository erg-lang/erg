# 元

元組類似於數組，但可以包含不同類型的對象。這樣的收藏稱為非等質收藏。與此相對，等質集合包括數組和集合。


```erg
t = (1, True, "a")
(i, b, s) = t
assert(i == 1 and b == True and s == "a")
```

元組可以以<gtr=“14”/>的形式檢索第 n 個元素。請注意，與 Python 不同，它不是。這是因為元組元素的訪問更接近於屬性，而不是方法（數組中的<gtr=“16”/>是方法）（編譯時檢查元素是否存在，類型可以根據 n 而變化）。


```erg
assert t.0 == 1
assert t.1 == True
assert t.2 == "a"
```

不嵌套時，括號是可選的。


```erg
t = 1, True, "a"
i, b, s = t
```

元組可以包含不同類型的對象，但不能包含數組之類的小版本。


```erg
t: ({1}, {2}, {3}) = (1, 2, 3)
(1, 2, 3).iter().map(x -> x + 1) # TypeError: type ({1}, {2}, {3}) has no method `.iter()`
# すべて同じ型の場合配列と同じように`(T; n)`で表せるが、これでもイテレーションは出來ない
t: (Int; 3) = (1, 2, 3)
assert (Int; 3) == (Int, Int, Int)
```

但是，非等質集合（如元組）可以通過上傳、Intersection 等轉換為等質集合（如數組）。這叫做等質化。


```erg
(Int, Bool, Str) can be [T; 3] | T :> Int, T :> Bool, T :> Str
```


```erg
t: (Int, Bool, Str) = (1, True, "a") # non-homogenous
a: [Int or Bool or Str; 3] = [1, True, "a"] # homogenous
_a: [Show; 3] = [1, True, "a"] # homogenous
_a.iter().map(x -> log x) # OK
t.try_into([Show; 3])?.iter().map(x -> log x) # OK
```

## 單位

具有 0 個元素的元組稱為單元。單位是一個值，但也指其類型本身。


```erg
unit = ()
(): ()
```

單元是所有元素 0 元組的超類。


```erg
() > (Int; 0)
() > (Str; 0)
```

此對象的用途包括參數、沒有返回值的過程等。 Erg 子例程必須具有參數和返回值。但是，在某些情況下，例如在過程中，可能會產生副作用，但沒有有意義的參數返回值。在這種情況下，單位作為“沒有意義的，形式上的值”來使用。


```erg
# ↓ 実はこの括弧はユニット
p!() =
    # `print!`は意味のある値を返さない
    print! "Hello, world!"
p!: () => ()
```

但是，Python 在這種情況下更傾向於使用而不是單位。在 Erg 中，如果一開始就確定不返回有意義的值（如過程），則返回<gtr=“19”/>；如果操作失敗，可能一無所獲（如元素檢索），則返回<gtr=“20”/>。

## 參數和元組

實際上，Erg 的所有對像都是一個參數，一個返回值。具有 N 個參數的子程序僅接受“一個具有 N 個元素的元組”作為參數。


```erg
# f x = ...は暗黙にf(x) = ...とみなされる
f x = x
assert f(1) == 1
f(1, 2, 3) # ArgumentError: f takes 1 positional argument but 3 were given
# 可変個の引數を受け取る
g x: Int, ...y: Int = y
assert (2, 3) == g 1, 2, 3
```

這將解釋函數的類型。


```erg
assert f in T: {(T,) -> T | T}
assert g in {(Int, ...(Int; N)) -> (Int; N) | N: Nat}
```

準確地說，函數的輸入不是元組，而是具有默認屬性的命名元組。這是一個特殊的元組，只能作為函數的參數使用，可以像記錄一樣命名並具有缺省值。


```erg
f(x: Int, y=0) = x + y
f: (Int, y=Int) -> Int

f(x=0, y=1)
f(y=1, x=0)
f(x=0)
f(0)
```

<p align='center'>
    <a href='./10_array.md'>Previous</a> | <a href='./12_dict.md'>Next</a>
</p>
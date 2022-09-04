# 具有默認參數的函數類型

首先，看默認自變量的使用例。


```erg
f: (Int, Int, z := Int) -> Int
f(x, y, z := 0) = x + y + z

g: (Int, Int, z := Int, w := Int) -> Int
g(x, y, z := 0, w := 1) = x + y + z + w

fold: ((Int, Int) -> Int, [Int], acc := Int) -> Int
fold(f, [], acc) = acc
fold(f, arr, acc := 0) = fold(f, arr[1..], f(acc, arr[0]))
assert fold(f, [1, 2, 3]) == 6
assert fold(g, [1, 2, 3]) == 8
```

之後的自變量為默認自變量。部分定型規則如下。


```erg
((X, y := Y) -> Z) <: (X -> Z)
((X, y := Y, ...) -> Z) <: ((X, ...) -> Z)
```

第 1 個意思是，有默認自變量的函數可以與沒有默認自變量的函數同等看待。第 2 個是可以省略任意的默認自變量的意思。
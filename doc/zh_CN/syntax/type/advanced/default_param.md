# 具有默认参数的函数类型

首先，看默认自变量的使用例。


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

之后的自变量为默认自变量。部分定型规则如下。


```erg
((X, y := Y) -> Z) <: (X -> Z)
((X, y := Y, ...) -> Z) <: ((X, ...) -> Z)
```

第 1 个意思是，有默认自变量的函数可以与没有默认自变量的函数同等看待。第 2 个是可以省略任意的默认自变量的意思。

# 帶默認參數的函數類型

首先，讓我們看一個使用默認參數的示例。

```python
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

`:=` 之后的參數是默認參數。
子類型規則如下:

```python
((X, y := Y) -> Z) <: (X -> Z)
((X, y := Y, ...) -> Z) <: ((X, ...) -> Z)
```

第一個意味著可以用沒有默認參數的函數來識別具有默認參數的函數。
第二個意味著可以省略任何默認參數。
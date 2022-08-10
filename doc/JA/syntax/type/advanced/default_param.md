# デフォルト引数付きの関数型

まず、デフォルト引数の使用例を見る。

```erg
f: (Int, Int, |= Int) -> Int
f(x, y, z |= 0) = x + y + z

g: (Int, Int, |= Int, Int) -> Int
g(x, y, z |= 0, w |= 1) = x + y + z + w

fold: ((Int, Int) -> Int, [Int], |= Int) -> Int
fold(f, [], acc) = acc
fold(f, arr, acc |= 0) = fold(f, arr[1..], f(acc, arr[0]))
assert fold(f, [1, 2, 3]) == 6
assert fold(g, [1, 2, 3]) == 8
```

`|=`以降の引数はデフォルト引数である。
部分型付け規則は以下の通り。

```erg
((X, |= Y) -> Z) < (X -> Z)
((X, |= Y, ...) -> Z) < ((X, |= ...) -> Z)
```

1番目は、デフォルト引数のある関数はない関数と同一視できる、という意味である。
2番目は、任意のデフォルト引数は省略できる、という意味である。

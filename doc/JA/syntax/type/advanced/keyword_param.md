# キーワード引数付き関数型

```erg
h(f) = f(y: 1, x: 2)
h: |T: Type|((y: Int, x: Int) -> T) -> T
```

キーワード引数付き関数の部分型付け規則は以下の通り。

```erg
((x: T, y: U) -> V) <: ((T, U) -> V)  # x, y are arbitrary keyword parameters
((y: U, x: T) -> V) <: ((x: T, y: U) -> V)
((x: T, y: U) -> V) <: ((y: U, x: T) -> V)
```

これは、キーワード引数は消去ないし入れ替えができるということを意味する。
しかし、両者を同時に行うことはできない。
すなわち、`(x: T, y: U) -> V`を`(U, T) -> V`にキャストすることはできない。
なお、キーワード引数がつくのはトップレベルのタプル内のみで、配列やネストしたタプルでキーワード引数は付かない。

```erg
Valid: [T, U] -> V
Invalid: [x: T, y: U] -> V
Valid: (x: T, ys: (U,)) -> V
Invalid: (x: T, ys: (y: U,)) -> V
```

# 無名関数(anonymous function)

無名関数は、関数オブジェクトを名付けずその場で生成するための文法である。

```erg
# `->`は無名関数演算子
# same as `f x, y = x + y`
f = (x, y) -> x + y
# same as `g(x, y: Int): Int = x + y`
g = (x, y: Int): Int -> x + y
```

引数が1つの場合は`()`を省略できる。

```erg
assert [1, 2, 3].map_collect(i -> i + 1) == [2, 3, 4]
assert ((i, j) -> [i, j])(1, 2) == [1, 2]
```

下の場合`0..9, (i -> ...)`であって`(0..9, i) -> ...`ではない。
`->`は左辺に一つだけ引数をとる。複数の引数は一つのタプルとして受け取る。

```erg
for 0..9, i: Int ->
    ...
```

無名関数では、空白による構文解釈の差異が存在する。

```erg
# この場合は`T(() -> Int)`と解釈される
i: T () -> Int
# この場合は(U()) -> Intと解釈される
k: U() -> Int
```

無名関数は引数なしでも使える。`=>`は無名プロシージャ演算子。

```erg
p! = () => print! "`p!` was called"
# `() ->`, `() =>`には`do`, `do!`という糖衣構文がある
# p! = do! print! "`p!` was called"
p!() # `p!` was called
```

引数なし関数は遅延初期化に使える。

```erg
time = import "time"
date = import "datetime"
now = if! True:
    do!:
        time.sleep! 1000
        date.now!()
    do date.new("1970", "1", "1", "00", "00")
```

型付け、パターンマッチもできる。このため、`match`関数はほとんど無名関数の力で実現されている。
`match`関数の引数に与える無名関数は上から順番にトライされる。ので、上の方は特殊なケースを、下に行くほど一般的なケースを記述する必要がある。順番を間違えると(可能な限り)コンパイラがWarningを出す。

```erg
n = (Complex or Ratio or Int).sample!()
i = match n:
    PI -> PI # 定数PIに等しい場合
    (i: 1..10) -> i # 1~10のIntの場合
    (i: Int) -> i # Intの場合
    (c: Complex) -> c.real() # Complexの場合。Int < Complexだが、フォールバックできる
    _ -> panic "cannot convert to Int" # 以上のいずれにも該当しない場合。matchは全パターンを網羅していなくてはならない
```

エラーハンドリングも`?`か`match`を使用して行うのが一般的である。

```erg
res: ParseResult Int
match res:
    i: Int -> i
    err: Error -> panic err.msg

res2: Result Int, Error
match res2:
    ok: Not Error -> log Typeof ok
    err: Error -> panic err.msg
```

## 無名多相関数

```erg
# same as id|T| x: T = x
id = |T| x: T -> x
```

<p align='center'>
    <a href='./20_naming_rule.md'>Previous</a> | <a href='./22_subroutine.md'>Next</a>
</p>

# 匿名函數（anonymous function）

匿名函數是一種語法，用於在不命名的情況下生成函數對象。


```erg
# `->`は無名関數演算子
# same as `f x, y = x + y`
f = (x, y) -> x + y
# same as `g(x, y: Int): Int = x + y`
g = (x, y: Int): Int -> x + y
```

如果只有一個參數，則可以省略。


```erg
assert [1, 2, 3].map_collect(i -> i + 1) == [2, 3, 4]
assert ((i, j) -> [i, j])(1, 2) == [1, 2]
```

在下面的情況下，它是，而不是<gtr=“12”/>。 <gtr=“13”/>在左邊只有一個參數。將多個參數視為單個元組。


```erg
for 0..9, i: Int ->
    ...
```

在未命名函數中，由於空格而存在語法差異。


```erg
# この場合は`T(() -> Int)`と解釈される
i: T () -> Int
# この場合は(U()) -> Intと解釈される
k: U() -> Int
```

不帶參數也可以使用匿名函數。


```erg
# `=>`は無名プロシージャ演算子
p! = () => print! "`p!` was called"
# `() ->`, `() =>`には`do`, `do!`という糖衣構文がある
# p! = do! print! "`p!` was called"
p!() # `p!` was called
```

無參數函數可用於延遲初始化。


```erg
time = import "time"
date = import "datetime"
now = if! True:
    do!:
        time.sleep! 1000
        date.now!()
    do date.new("1970", "1", "1", "00", "00")
```

還可以進行打字和模式匹配。因此，函數幾乎是通過無名函數的力量來實現的。函數參數中的無名函數將從上到下依次嘗試。所以，上面的需要描述特殊情況，越往下越需要描述一般情況。如果順序錯誤（盡可能），編譯器將發出警告。


```erg
n = (Complex or Ratio or Int).sample!()
i = match n:
    PI -> PI # 定數PIに等しい場合
    (i: 1..10) -> i # 1~10のIntの場合
    (i: Int) -> i # Intの場合
    (c: Complex) -> c.real() # Complexの場合。 Int < Complexだが、フォールバックできる
    _ -> panic "cannot convert to Int" # 以上のいずれにも該當しない場合。 matchは全パターンを網羅していなくてはならない
```

錯誤處理也通常使用或<gtr=“17”/>。


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

## 無名多相關數


```erg
# same as id|T| x: T = x
id = |T| x: T -> x
```

<p align='center'>
    <a href='./20_naming_rule.md'>Previous</a> | <a href='./22_subroutine.md'>Next</a>
</p>
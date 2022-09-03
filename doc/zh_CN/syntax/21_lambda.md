# 匿名函数（anonymous function）

匿名函数是一种语法，用于在不命名的情况下生成函数对象。


```erg
# `->`は無名関数演算子
# same as `f x, y = x + y`
f = (x, y) -> x + y
# same as `g(x, y: Int): Int = x + y`
g = (x, y: Int): Int -> x + y
```

如果只有一个参数，则可以省略。


```erg
assert [1, 2, 3].map_collect(i -> i + 1) == [2, 3, 4]
assert ((i, j) -> [i, j])(1, 2) == [1, 2]
```

在下面的情况下，它是，而不是<gtr=“12”/>。<gtr=“13”/>在左边只有一个参数。将多个参数视为单个元组。


```erg
for 0..9, i: Int ->
    ...
```

在未命名函数中，由于空格而存在语法差异。


```erg
# この場合は`T(() -> Int)`と解釈される
i: T () -> Int
# この場合は(U()) -> Intと解釈される
k: U() -> Int
```

不带参数也可以使用匿名函数。


```erg
# `=>`は無名プロシージャ演算子
p! = () => print! "`p!` was called"
# `() ->`, `() =>`には`do`, `do!`という糖衣構文がある
# p! = do! print! "`p!` was called"
p!() # `p!` was called
```

无参数函数可用于延迟初始化。


```erg
time = import "time"
date = import "datetime"
now = if! True:
    do!:
        time.sleep! 1000
        date.now!()
    do date.new("1970", "1", "1", "00", "00")
```

还可以进行打字和模式匹配。因此，函数几乎是通过无名函数的力量来实现的。函数参数中的无名函数将从上到下依次尝试。所以，上面的需要描述特殊情况，越往下越需要描述一般情况。如果顺序错误（尽可能），编译器将发出警告。


```erg
n = (Complex or Ratio or Int).sample!()
i = match n:
    PI -> PI # 定数PIに等しい場合
    (i: 1..10) -> i # 1~10のIntの場合
    (i: Int) -> i # Intの場合
    (c: Complex) -> c.real() # Complexの場合。Int < Complexだが、フォールバックできる
    _ -> panic "cannot convert to Int" # 以上のいずれにも該当しない場合。matchは全パターンを網羅していなくてはならない
```

错误处理也通常使用或<gtr=“17”/>。


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

## 无名多相关数


```erg
# same as id|T| x: T = x
id = |T| x: T -> x
```

<p align='center'>
    <a href='./20_naming_rule.md'>Previous</a> | <a href='./22_subroutine.md'>Next</a>
</p>

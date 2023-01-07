# Lambda

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/22_lambda.md%26commit_hash%3Dc8932f8fd75cc86f67421bb6b160fffaf7acdd94)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/22_lambda.md&commit_hash=c8932f8fd75cc86f67421bb6b160fffaf7acdd94)

匿名函数是一种无需命名即可动态创建函数对象的语法

```python
# `->` 是匿名函数操作符
# 同 `f x, y = x + y`
f = (x, y) -> x + y
# same as `g(x, y: Int): Int = x + y`
g = (x, y: Int): Int -> x + y
```

如果只有一个参数，您可以省略 `()`

```python
assert [1, 2, 3].map_collect(i -> i + 1) == [2, 3, 4]
assert ((i, j) -> [i, j])(1, 2) == [1, 2]
```

在下面的情况下，它是 `0..9, (i -> ...)` 而不是 `(0..9, i) -> ...`
`->` 在左侧只接受一个参数。多个参数作为单个元组接收

```python
for 0..9, i: Int ->
    ...
```

在匿名函数中，由于空格，解析存在差异

```python
# 在这种情况下，解释为 `T(() -> Int)`
i: T() -> Int
# 在这种情况下，它被解释为 (U()) -> Int
k: U() -> Int
```

匿名函数可以不带参数使用

```python
# `=>` 是一个匿名过程操作符
p! = () => print! # `p!` 被调用
# `() ->`, `() =>` 有语法糖 `do`, `do!`
# p! = do! print! "`p!` 被调用
p!() # `p!` 被调用
```

无参数函数可用于延迟初始化

```python
time = import "time"
date = import "datetime"
now = if! True:
    do!:
        time. sleep! 1000
        date.now!()
    do date.new("1970", "1", "1", "00", "00")
```

您还可以键入和模式匹配。正因为如此，`match` 函数大多是借助匿名函数的力量来实现的
作为 `match` 函数的参数给出的匿名函数从顶部开始按顺序尝试。因此，您应该在顶部描述特殊情况，在底部描述更一般的情况。如果你弄错了顺序，编译器会发出警告(如果可能的话)

```python
n = (Complex or Ratio or Int).sample!()
i = matchn:
    PI -> PI # 如果等于常数 PI
    For (i: 1..10) -> i # 整数从 1 到 10
    (i: Int) -> i # Int
    (c: Complex) -> c.real() # 对于复杂。Int < Complex，但可以回退
    _ -> panic "cannot convert to Int" # 如果以上都不适用。match 必须涵盖所有模式
```

错误处理通常也使用 `?` 或 `match` 完成

```python
res: ParseResult Int
matchres:
    i: Int -> i
    err: Error -> panic err.msg

res2: Result Int, Error
match res2:
    ok: Not Error -> log Type of ok
    err: Error -> panic err.msg
```

## 匿名多相关系数

```python
# 与此相同 id|T|x: T = x
id = |T| x: T -> x
```

<p align='center'>
    <a href='./21_naming_rule.md'>上一页</a> | <a href='./23_subroutine.md'>下一页</a>
</p>
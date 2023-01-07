# Lambda

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/21_lambda.md%26commit_hash%3D20aa4f02b994343ab9600317cebafa2b20676467)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/21_lambda.md&commit_hash=20aa4f02b994343ab9600317cebafa2b20676467)

匿名函數是一種無需命名即可動態創建函數對象的語法

```python
# `->` 是匿名函數操作符
# 同 `f x, y = x + y`
f = (x, y) -> x + y
# same as `g(x, y: Int): Int = x + y`
g = (x, y: Int): Int -> x + y
```

如果只有一個參數，您可以省略 `()`

```python
assert [1, 2, 3].map_collect(i -> i + 1) == [2, 3, 4]
assert ((i, j) -> [i, j])(1, 2) == [1, 2]
```

在下面的情況下，它是 `0..9, (i -> ...)` 而不是 `(0..9, i) -> ...`
`->` 在左側只接受一個參數。多個參數作為單個元組接收

```python
for 0..9, i: Int ->
    ...
```

在匿名函數中，由于空格，解析存在差異

```python
# 在這種情況下，解釋為 `T(() -> Int)`
i: T() -> Int
# 在這種情況下，它被解釋為 (U()) -> Int
k: U() -> Int
```

匿名函數可以不帶參數使用

```python
# `=>` 是一個匿名過程操作符
p! = () => print! # `p!` 被調用
# `() ->`, `() =>` 有語法糖 `do`, `do!`
# p! = do! print! "`p!` 被調用
p!() # `p!` 被調用
```

無參數函數可用于延遲初始化

```python
time = import "time"
date = import "datetime"
now = if! True:
    do!:
        time. sleep! 1000
        date.now!()
    do date.new("1970", "1", "1", "00", "00")
```

您還可以鍵入和模式匹配。正因為如此，`match` 函數大多是借助匿名函數的力量來實現的
作為 `match` 函數的參數給出的匿名函數從頂部開始按順序嘗試。因此，您應該在頂部描述特殊情況，在底部描述更一般的情況。如果你弄錯了順序，編譯器會發出警告(如果可能的話)

```python
n = (Complex or Ratio or Int).sample!()
i = matchn:
    PI -> PI # 如果等于常數 PI
    For (i: 1..10) -> i # 整數從 1 到 10
    (i: Int) -> i # Int
    (c: Complex) -> c.real() # 對于復雜。Int < Complex，但可以回退
    _ -> panic "cannot convert to Int" # 如果以上都不適用。match 必須涵蓋所有模式
```

錯誤處理通常也使用 `?` 或 `match` 完成

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

## 匿名多相關系數

```python
# 與此相同 id|T|x: T = x
id = |T| x: T -> x
```

<p align='center'>
    <a href='./20_naming_rule.md'>上一頁</a> | <a href='./22_subroutine.md'>下一頁</a>
</p>
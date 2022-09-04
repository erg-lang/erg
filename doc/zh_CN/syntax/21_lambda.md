# anonymous function

Anonymous functions are a syntax for creating function objects on the fly without naming them.

```python
# `->` is an anonymous function operator
# same as `f x, y = x + y`
f = (x, y) -> x + y
# same as `g(x, y: Int): Int = x + y`
g = (x, y: Int): Int -> x + y
```

You can omit the `()` if there is only one argument.

```python
assert [1, 2, 3].map_collect(i -> i + 1) == [2, 3, 4]
assert ((i, j) -> [i, j])(1, 2) == [1, 2]
```

In the case below it is `0..9, (i -> ...)` and not `(0..9, i) -> ...`.
`->` takes only one argument on the left side. Multiple arguments are received as a single tuple.

```python
for 0..9, i: Int ->
    ...
```

In anonymous functions, there is a difference in parsing due to whitespace.

```python
# In this case, interpreted as `T(() -> Int)`
i: T() -> Int
# in this case it is interpreted as (U()) -> Int
k: U() -> Int
```

Anonymous functions can be used without arguments.

```python
# `=>` is an anonymous procedure operator
p! = () => print! "`p!` was called"
# `() ->`, `() =>` have syntax sugar `do`, `do!`
# p! = do! print! "`p!` was called"
p!() # `p!` was called
```

No-argument functions can be used for lazy initialization.

```python
time = import "time"
date = import "datetime"
now = if! True:
    do!:
        time. sleep! 1000
        date.now!()
    do date.new("1970", "1", "1", "00", "00")
```

You can also type and pattern match. Because of this, the `match` function is mostly implemented with the power of anonymous functions.
Anonymous functions given as arguments to the `match` function are tried in order from the top. So, you should describe the special cases at the top and the more general cases at the bottom. If you get the order wrong, the compiler will issue a warning (if possible).

```python
n = (Complex or Ratio or Int).sample!()
i = matchn:
    PI -> PI # if equal to constant PI
    For (i: 1..10) -> i # Int from 1 to 10
    (i: Int) -> i # Int
    (c: Complex) -> c.real() # For Complex. Int < Complex, but can fallback
    _ -> panic "cannot convert to Int" # If none of the above apply. match must cover all patterns
```

Error handling is also generally done using `?` or `match`.

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

## Anonymous polycorrelation coefficient

```python
# same as id|T| x: T = x
id = |T| x: T -> x
```

<p align='center'>
    <a href='./20_naming_rule.md'>上一页</a> | <a href='./22_subroutine.md'>下一页</a>
</p>
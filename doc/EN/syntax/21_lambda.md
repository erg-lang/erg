# Anonymous Function

An anonymous function is a syntax for creating function objects on the fly without naming them.

```erg
# `->` is the anonymous function operator
# same as `f x, y = x + y`
f = (x, y) -> x + y
# same as `g(x, y: Int): Int = x + y`
g = (x, y: Int): Int -> x + y
```

You can omit `()` if there is only one argument.

```erg
assert [1, 2, 3].map_collect(i -> i + 1) == [2, 3, 4].
assert ((i, j) -> [i, j])(1, 2) == [1, 2].
```

In the case below `0..9, (i -> ...)`, not `(0..9, i) -> ...`.
`->` takes only one argument on the left side. Multiple arguments are taken as a single tuple.

```erg
for 0..9, i: Int ->
    ...
```

For anonymous functions, there is a difference in syntactic interpretation due to whitespace.

```erg
# In this case, it is interpreted as ``T(() -> Int)``.
i: T () -> Int
# In this case, it is interpreted as (U()) -> Int
k: U() -> Int
```

Anonymous functions can be used without arguments. `=>` is an anonymous procedure operator.

```erg
p!= () => print!"`p!` was called"
# `() ->`, `() =>` have the sugar-coated constructs `do`, `do!`.
# p! = do! print! "`p!` was called"
p!() # `p!` was called
```

Argumentless functions can be used for lazy initialization.

```erg
time = import "time"
date = import "datetime"
now = if! True:
    do!
        time.sleep!
        date.now!()
    do date.new("1970", "1", "1", "00", "00")
```

Typing and pattern matching can also be done. For this reason, the ``match`` function is realized almost entirely by the power of anonymous functions.
The anonymous functions given as arguments to the ``match`` function are tried in order from the top. So, it is necessary to describe special cases at the top and more general cases as you go down. If you get the order wrong (as far as possible), the compiler will issue a Warning.

```erg
n = (Complex or Ratio or Int).sample!
i = match n:
    PI -> PI # if equal to constant PI
    (i: 1..10) -> i # if 1~10 Int
    (i: Int) -> i # for Int
    (c: Complex) -> c.real() # case of Complex, Int < Complex, but can fall back
    _ -> panic "cannot convert to Int" # none of the above; match must cover all patterns
```

Error handling can also be done using `?` or `match`.

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

## Anonymous Polymorphic Function

```erg
# same as id|T| x: T = x
id = |T| x: T -> x
```

<p align='center'>
    <a href='. /20_naming_rule.md'>Previous</a> | <a href='. /22_subroutine.md'>Next</a>
</p>

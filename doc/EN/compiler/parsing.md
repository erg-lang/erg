# Parsing the Erg language

## Treatment of whitespace

A peculiarity of Erg's grammar is that it is space-sensitive.
This is to compensate for the loss of expressiveness caused by the omission of `()`. A similar syntax is found in Nim, which also allows the omission of `()`.

```erg
f +1 == f(+1)
f + 1 == `+`(f, 1)
f (1,) == f((1,))
f(1,) == f(1)
(f () -> ...) == f(() -> ...)
(f() -> ...) == (f() -> ...)
```

## Left-hand side value, right-hand side value

In Erg, left-hand side values are not as simple as the left-hand side of `=`.
In fact, there is (very confusingly) a right-sided value on the left side of `=`, and a left-sided value on the right side of `=`.
There can even be a left-side value within a right-side value.

```erg
# i is the left-hand side value, Array(Int) and [1, 2, 3] are the right-hand side values
i: Array(Int) = [1, 2, 3]
# `[1, 2, 3].iter().map i -> i + 1` is the right-hand side value, but i to the left of -> is the left-hand side value
a = [1, 2, 3].iter().map i -> i + 1
# {x = 1; y = 2} is the right side value, but x, y are the left side values
r = {x = 1; y = 2}
```

The precise definition of left- and right-hand side values is "right-hand side value if it is evaluable, otherwise left-hand side value".
As an example, consider the code ``i = 1; i``, where the second `i` is a right-sided value because it is evaluable, but the first `i` is a left-sided value.

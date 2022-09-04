# Closure

Erg subroutines have a feature called a "closure" that captures external variables.

``` erg
outer = 1
f x = outer + x
assert f(1) == 2
```

As with immutable objects, mutable objects can also be captured.

``` erg
sum = !0
for! 1..10, i =>
    sum.add!i
assert sum == 45

p!x=
    sum.add!x
p!(1)
assert sum == 46
```

Note, however, that functions cannot capture mutable objects.
If a mutable object can be referenced in a function, you can write code like the following.

``` erg
# !!! This code actually gives an error !!!
i = !0
f x = i + x
assert f 1 == 1
i.add! 1
assert f 1 == 2
```

The function should return the same value for the same arguments, but the assumption is broken.
Note that `i` is evaluated only at call time.

Call `.clone` if you want the contents of the mutable object at the time the function was defined.

``` erg
i = !0
immut_i = i.clone().freeze()
fx = immut_i + x
assert f 1 == 1
i.add! 1
assert f 1 == 1
```

## avoid mutable state, functional programming

``` erg
# Erg
sum = !0
for! 1..10, i =>
    sum.add!i
assert sum == 45
```

The equivalent program above can be written in Python as follows:

```python
# Python
sum = 0
for i in range(1, 10):
    sum += i
assert sum == 45
```

However, Erg recommends a simpler notation.
Instead of carrying around state using subroutines and mutable objects, use a style of localizing state using functions. This is called functional programming.

``` erg
# Functional style
sum = (1..10).sum()
assert sum == 45
```

The code above gives exactly the same result as before, but you can see that this one is much simpler.

The `fold` function can be used to do more than sum.
`fold` is an iterator method that executes the argument `f` for each iteration.
The initial value of the counter that accumulates results is specified in `init` and accumulated in `acc`.

``` erg
# start with 0, result will
sum = (1..10).fold(init: 0, f: (acc, i) -> acc + i)
assert sum == 45
```

Erg is designed to be a natural succinct description of programming with immutable objects.

<p align='center'>
    <a href='./22_subroutine.md'>Previous</a> | <a href='./24_module.md'>Next</a>
</p>
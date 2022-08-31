# Closure

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/23_closure.md%26commit_hash%3D21e8145e83fb54ed77e7631deeee8a7e39b028a3)
](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/23_closure.md&commit_hash=21e8145e83fb54ed77e7631deeee8a7e39b028a3)

Erg subroutines have a "closure" feature that captures external variables.

```erg
outer = 1
f x = outer + x
assert f(1) == 2
```

Like immutable objects, mutable objects can also be captured.

```erg
sum = !0
for! 1..10, i =>
    sum.add!
assert sum == 45

p! x =
    sum.add!
p!(1)
assert sum == 46
```

Note, however, that functions cannot capture mutable objects.
If mutable objects could be referenced in a function, the following code could be written.

```erg
# !!! This code is actually an error !!!
i = !0
f x = i + x
assert f 1 == 1
i.add! 1
assert f 1 == 2
```

The function should return the same value for the same argument, but that assumption has been violated.
Note that ``i`` is evaluated for the first time at call time.

If you want the contents of a mutable object at the time of the function definition, `.clone` it.

```erg
i = !0
immut_i = i.clone().freeze()
f x = immut_i + x
assert f 1 == 1
i.add! 1
assert f 1 == 1
```

## Avoiding Mutable States, Functional Programming

```erg
## Erg
sum = !0
for! 1..10, i =>
    sum.add!
assert sum == 45
```

The equivalent program above can be written in Python as follows.

```python
# Python
sum = 0
for i in range(1, 10):
    sum += i
assert sum == 45
```

However, Erg recommends a simpler way of writing.
Instead of using subroutines and mutable objects to carry around state, the style is to localize the state using functions. This is called functional programming.

```erg
# Functional style
sum = (1..10).sum()
assert sum == 45
```

The above code produces exactly the same result as the previous one, but it can be seen that this one is much simpler.

The `fold` function can be used to perform a variety of operations other than summing.
`fold` is an iterator method that executes the argument `f` for each iteration.
The initial value of the counter to accumulate the results is specified by `init` and accumulated in `acc`.

```erg
# start with 0, result will
sum = (1..10).fold(init: 0, f: (acc, i) -> acc + i)
assert sum == 45
```

Erg is designed to be a natural and concise description of programming with invariant objects.

<p align='center'>
    <a href='./22_subroutine.md'>Previous</a> | <a href='./24_module.md'>Next</a>
</p>

# pipeline operator

Pipeline operators are used like this:

```python
assert f(g(x)) == (x |> g |> f)
assert f(g(x, y)) == ((x, y) |> g |> f)
```

In other words, the order `Callable(object)` can be changed to `object |> Callable`.
The pipeline operator can also be used on 方法. For 方法, `object.method(args)` changes to `object |>.method(args)`.
It looks like just more `|>`, but since the bond strength is low, you may be able to reduce the amount of `()`.

```python
rand = -1.0..1.0 |>.sample!()
log rand # 0.2597...

1+1*2 |>.times do log("a", end := "") # aaa

evens = 1..100 |>.iter |>.filter i -> i % 2 == 0 |>.collect Array
# When implemented without the pipeline operator,
_evens = (1..100).iter().filter(i -> i % 2 == 0).collect(Array)
# or
__evens = 1..100 \
    .iter() \
    .filter i -> i % 2 == 0 \
    .collect Array
```

<p align='center'>
    <a href='./30_error_handling.md'>Previous</a> | <a href='./32_integration_with_Python.md'>Next</a>
</p>
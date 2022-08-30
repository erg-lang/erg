# Pipeline Operator

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/31_pipeline.md%26commit_hash%3Da84ab773d007af040d8ad1854bbeb38d136d9c46)
](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/31_pipeline.md&commit_hash=a84ab773d007af040d8ad1854bbeb38d136d9c46)

The pipeline operator is used like this:

``` erg
assert f(g(x)) == (x |> g |> f)
assert f(g(x, y)) == ((x, y) |> g |> f)
```

In other words, the order `Callable(object)` can be changed to `object |> Callable`.
Pipeline operators can also be used on methods. For methods, `object.method(args)` changes to `object |>.method(args)`.
It looks like just an increase in `|>`, but since the bond strength is low, the amount of `()` may be reduced.

``` erg
rand = -1.0..1.0 |>.sample!()
log rand # 0.2597...
1+1*2 |>.times do log("a", end: "") # aaa
# without `|>`, the following will be `evens = (1..100).iter().filter(i -> i % 2 == 0).collect(Array)`
evens = 1..100 |>.iter |>.filter i -> i % 2 == 0 |>.collect Array
# or
_evens = 1..100\
    .iter() \
    .filter i -> i % 2 == 0 \
    .collect Array
```

<p align='center'>
    <a href='./30_error_handling.md'>Previous</a> | <a href='./32_integration_with_Python.md'>Next</a>
</p>

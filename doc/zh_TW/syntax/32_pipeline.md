# 管道運算符

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/32_pipeline.md%26commit_hash%3Dc8932f8fd75cc86f67421bb6b160fffaf7acdd94)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/32_pipeline.md&commit_hash=c8932f8fd75cc86f67421bb6b160fffaf7acdd94)

管道運算符的使用方式如下:

```python
assert f(g(x)) == (x |> g() |> f())
assert f(g(x, y)) == (x |> g(y) |> f())
```

換句話說，`Callable(object)`的順序可以更改為`object |> Callable()`
管道運算符也可用于方法。對于方法，`object.method(args)`更改為`object |>.method(args)`
它看起來只是更多的`|>`，但由于粘合強度較低，您可以減少`()`的數量

```python
rand = -1.0..1.0 |>.sample!()
log rand # 0.2597...

1+1*2 |>.times do log("a", end := "") # aaa

evens = 1..100 |>.iter() |>.filter i -> i % 2 == 0 |>.collect Array
# 在沒有管道操作符的情況下實現，
_evens = (1..100).iter().filter(i -> i % 2 == 0).collect(Array)
# or
__evens = 1..100 \
    .iter() \
    .filter i -> i % 2 == 0 \
    .collect Array
```

<p align='center'>
    <a href='./31_error_handling.md'>上一頁</a> | <a href='./33_integration_with_Python.md'>下一頁</a>
</p>

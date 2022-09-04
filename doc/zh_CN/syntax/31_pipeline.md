# 管道运算符

管道运算符的使用方式如下：

```python
assert f(g(x)) == (x |> g |> f)
assert f(g(x, y)) == ((x, y) |> g |> f)
```

换句话说，`Callable(object)` 的顺序可以更改为 `object |> Callable`。
管道运算符也可用于方法。 对于方法，`object.method(args)` 更改为 `object |>.method(args)`。
它看起来只是更多的`|>`，但由于粘合强度较低，您可以减少`()`的数量。

```python
rand = -1.0..1.0 |>.sample!()
log rand # 0.2597...

1+1*2 |>.times do log("a", end := "") # aaa

evens = 1..100 |>.iter |>.filter i -> i % 2 == 0 |>.collect Array
# 在没有管道操作符的情况下实现，
_evens = (1..100).iter().filter(i -> i % 2 == 0).collect(Array)
# or
__evens = 1..100 \
    .iter() \
    .filter i -> i % 2 == 0 \
    .collect Array
```

<p align='center'>
    <a href='./30_error_handling.md'>上一页</a> | <a href='./32_integration_with_Python.md'>下一页</a>
</p>
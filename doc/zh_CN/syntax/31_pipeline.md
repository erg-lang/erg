# 流水线运算符

按如下方式使用管线运算符。


```erg
assert f(g(x)) == (x |> g |> f)
assert f(g(x, y)) == ((x, y) |> g |> f)
```

这意味着你可以将的顺序更改为<gtr=“4”/>。也可以对方法使用管线运算符。对于方法，<gtr=“5”/>更改为<gtr=“6”/>。虽然它看起来只是增加了<gtr=“7”/>，但由于耦合强度较低，可能会减少<gtr=“8”/>的量。


```erg
rand = -1.0..1.0 |>.sample!()
log rand # 0.2597...

1+1*2 |>.times do log("a", end := "") # aaa

evens = 1..100 |>.iter |>.filter i -> i % 2 == 0 |>.collect Array
# パイプライン演算子を使わずに実装する場合、
_evens = (1..100).iter().filter(i -> i % 2 == 0).collect(Array)
# または
__evens = 1..100 \
    .iter() \
    .filter i -> i % 2 == 0 \
    .collect Array
```

<p align='center'>
    <a href='./30_error_handling.md'>Previous</a> | <a href='./32_integration_with_Python.md'>Next</a>
</p>

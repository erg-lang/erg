# 流水線運算符

按如下方式使用管線運算符。


```erg
assert f(g(x)) == (x |> g |> f)
assert f(g(x, y)) == ((x, y) |> g |> f)
```

這意味著你可以將的順序更改為<gtr=“4”/>。也可以對方法使用管線運算符。對於方法，<gtr=“5”/>更改為<gtr=“6”/>。雖然它看起來只是增加了<gtr=“7”/>，但由於耦合強度較低，可能會減少<gtr=“8”/>的量。


```erg
rand = -1.0..1.0 |>.sample!()
log rand # 0.2597...

1+1*2 |>.times do log("a", end := "") # aaa

evens = 1..100 |>.iter |>.filter i -> i % 2 == 0 |>.collect Array
# パイプライン演算子を使わずに実裝する場合、
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
# パイプライン演算子

パイプライン演算子は、次のように使います。

```erg
assert f(g(x)) == (x |> g |> f)
assert f(g(x, y)) == ((x, y) |> g |> f)
```

つまり、`Callable(object)`という順序を`object |> Callable`に変えられます。
パイプライン演算子はメソッドに対しても使えます。メソッドの場合、`object.method(args)`が`object |>.method(args)`と変わります。
単に`|>`が増えただけにも見えるが、結合強度が低めなので`()`の量を減らせる場合があります。

```erg
rand = -1.0..1.0 |>.sample!()
log rand # 0.2597...

1+1*2 |>.times do log("a", end: "") # aaa

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

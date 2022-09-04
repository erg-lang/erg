# パイプライン演算子

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/31_pipeline.md%26commit_hash%3Deccd113c1512076c367fb87ea73406f91ff83ba7)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/31_pipeline.md&commit_hash=eccd113c1512076c367fb87ea73406f91ff83ba7)

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

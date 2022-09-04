# モジュール

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/24_module.md%26commit_hash%3Deccd113c1512076c367fb87ea73406f91ff83ba7)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/24_module.md&commit_hash=eccd113c1512076c367fb87ea73406f91ff83ba7)

Ergでは、ファイル自体を1つのレコードとみなすことができます。これをモジュールと呼びます。

```python: foo.er
# foo.er
.i = 1
```

```python
# fooモジュールを定義するのはこのレコードを定義するのとほとんど同じ
foo = {.i = 1}
```

```python: bar.er
# bar.er
foo = import "foo"
print! foo # <module 'foo'>
assert foo.i == 1
```

モジュール型はレコード型でもあるので、分解代入が可能です。

```python
{sin; cos; ...} = import "math"
```

## モジュールの可視性

```console
└─┬ ./src
  ├─ lib.er
  ├─ foo.er
  ├─ bar.er
  └─┬ bar
    ├─ baz.er
    └─ qux.er
```

<p align='center'>
    <a href='./23_closure.md'>Previous</a> | <a href='./25_object_system.md'>Next</a>
</p>

# モジュール


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

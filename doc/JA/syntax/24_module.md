# モジュール

Ergでは、ファイル自体を1つのレコードとみなすことができます。これをモジュールと呼びます。

```erg: foo.er
# foo.er
.i = 1
```

```erg
# fooモジュールを定義するのはこのレコードを定義するのとほとんど同じ
foo = {.i = 1}
```

```erg: bar.er
# bar.er
foo = import "foo"
print! foo # <module 'foo'>
assert foo.i == 1
```

モジュール型はレコード型でもあるので、分解代入が可能です。

```erg
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
    <a href='./23_scope.md'>Previous</a> | <a href='./25_object_system.md'>Next</a>
</p>

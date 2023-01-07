# モジュール

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/24_module.md%26commit_hash%3Dfba8b193ce4270cb8c9236c4ed7bb8b2497af3fd)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/24_module.md&commit_hash=fba8b193ce4270cb8c9236c4ed7bb8b2497af3fd)

Ergでは、ファイル自体を1つのレコードとみなすことができます。これをモジュールと呼びます。

```python,checker_ignore
# foo.er
.i = 1
```

```python
# fooモジュールを定義するのはこのレコードを定義するのとほとんど同じ
foo = {.i = 1}
```

```python,checker_ignore
# bar.er
foo = import "foo"
print! foo # <module 'foo'>
assert foo.i == 1
```

モジュール型はレコード型でもあるので、分解代入が可能です。
モジュールの場合は最後の`...`を省略できます。

```python
# same as {sin; cos; ...} = import "math"
{sin; cos} = import "math"
```

## モジュールの可視性

ファイルだけでなく、ディレクトリもモジュールとなりえます。
ただしデフォルトでErgはディレクトリをErgモジュールとしては認識しません。認識させるには、`__init__.er`という名前のファイルを作成します。
`__init__.er`はPythonの`__init__.py`と同じようなものです。

```console
└─┬ bar
  └─ __init__.er
```

これで、`bar`ディレクトリはモジュールとして認識されます。`bar`内にあるファイルが`__init__.er`だけならばあまりディレクトリ構造にする意味はありませんが、複数のモジュールを束ねて一つのモジュールとしたい場合は便利です。すなわち、このような場合です。

```console
└─┬ bar
  ├─ __init__.er
  ├─ baz.er
  └─ qux.er
```

`bar`ディレクトリの外側からは以下のようにして使用できます。

```python
bar = import "bar"

bar.baz.p!()
bar.qux.p!()
```

`__init__.er`は単にディレクトリをモジュールとして機能させるだけのマーカーではなく、モジュールの可視性を制御する役割も持ちます。

```python
# __init__.er

# `./`はカレントディレクトリを指す。なくても良い
.baz = import "./baz"
qux = import "./qux"

.f x =
    .baz.f ...
.g x =
    qux.f ...
```

外から`bar`モジュールをインポートしたとき、`baz`モジュールはアクセス可能ですが、`qux`モジュールはアクセス不可能になります。

## モジュールの循環参照

Ergでは、モジュール間の循環的な依存関係を定義することができます。

```python
# foo.er
bar = import "bar"

print! bar.g 1
.f x = x
```

```python
# bar.er
foo = "foo "をインポート

print! foo.f 1
.g x = x
```

しかし、手続き呼び出しによって作られた変数は、循環参照モジュールで定義することはできません。
これは、Ergが依存関係に従って定義の順番を並べ替えるからです。

```python,compile_fail
# foo.er
bar = import "bar"

print! bar.x
.x = g!(1) # ModuleError: 手続き呼び出しで作られた変数は、循環参照モジュールで定義できない
```

```python
# bar.er
foo = import "foo"

print! foo.x
.x = 0
```

また、エントリポイントであるErgモジュール（すなわち `__name__ == "__main__"` であるモジュール）は循環参照の対象になることはできません。

<p align='center'>
    <a href='./23_closure.md'>Previous</a> | <a href='./25_object_system.md'>Next</a>
</p>

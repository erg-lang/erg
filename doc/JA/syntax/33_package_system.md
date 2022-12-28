# パッケージシステム

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/33_package_system.md%26commit_hash%3D20aa4f02b994343ab9600317cebafa2b20676467)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/33_package_system.md&commit_hash=20aa4f02b994343ab9600317cebafa2b20676467)

Ergのパッケージはアプリケーションであるappパッケージとライブラリであるlibパッケージに大別できます。
appパッケージのエントリポイントは`src/app.er`です。`app.er`内に定義された`main`関数が実行されます。
libパッケージのエントリポイントは`src/lib.er`です。パッケージをインポートすることは`lib.er`をインポートすることと等価になります。

パッケージにはモジュールという下位構造があります。Ergにおいてモジュールとはすなわち、Ergファイルもしくはそれで構成されたディレクトリです。外部のErgファイル/ディレクトリはモジュールオブジェクトとして操作可能な対象になるのです。

ディレクトリをモジュールとして認識させるには、ディレクトリ内に`__init__.er`ファイルを置く必要があります。
これはPythonの`__init__.py`と同じようなものです。

例として、以下のようなディレクトリ構成を考えてみましょう。

```console
└─┬ ./src
  ├─ app.er
  ├─ foo.er
  └─┬ bar
    ├─ __init__.er
    ├─ baz.er
    └─ qux.er
```

`app.er`では`foo`モジュールと`bar`モジュールをインポートできます。`bar`ディレクトリがモジュールとして認識できるのは`__init__.er`ファイルがあるためです。
`foo`モジュールはファイルからなるモジュールで、`bar`モジュールはディレクトリからなるモジュールです。`bar`モジュールはさらに`baz`, `qux`モジュールを内部に持ちます。
このモジュールは単に`bar`モジュールの属性であり、`app.er`からは以下のようにアクセスできます。

```python
# app.er
foo = import "foo"
bar = import "bar"
baz = bar.baz
# または`baz = import "bar/baz"`

main args =
    ...
```

サブモジュールにアクセスするための区切り文字が`/`であることに注意してください。これは、`bar.baz.er`のようなファイル名があり得るためです。
しかしこのようなファイル名は推奨されません。Ergでは`.er`の直前の識別子、プレフィックスが意味を持つためです。
例えば、テスト用のモジュールです。`.test.er`で終わるファイルは(ホワイトボックス)テスト用のモジュールであり、テスト実行時に`@Test`でデコレーションされたサブルーチンが実行されます。

```console
└─┬ ./src
  ├─ app.er
  ├─ foo.er
  └─ foo.test.er
```

```python
# app.er
foo = import "foo"

main args =
    ...
```

また、`__init__.er`内でre-importされていないモジュールはプライベートモジュールであり、同一ディレクトリ内のモジュールからしかアクセスできません。

```console
└─┬
  ├─ foo.er
  └─┬ bar
    ├─ __init__.er
    ├─ baz.er
    └─ qux.er
```

```python
# __init__.py
.qux = import "qux" # this is public
```

```python,checker_ignore
# foo.er
bar = import "bar"
bar.qux
bar.baz # AttributeError: module 'baz' is private
```

```python
# qux.er
baz = import "baz"
```

<p align='center'>
    <a href='./32_integration_with_Python.md'>Previous</a> | <a href='./34_generator.md'>Next</a>
</p>

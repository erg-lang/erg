# パッケージシステム

Ergのパッケージはアプリケーションであるappパッケージとライブラリであるlibパッケージに大別できます。
appパッケージのエントリポイントは`src/app.er`です。`app.er`内に定義された`main`関数が実行されます。
libパッケージのエントリポイントは`src/lib.er`です。パッケージをインポートすることは`lib.er`をインポートすることと等価になります。

パッケージにはモジュールという下位構造があります。Ergにおいてモジュールとはすなわち、Ergファイルもしくはそれで構成されたディレクトリです。外部のErgファイル/ディレクトリはモジュールオブジェクトとして操作可能な対象になるのです。

ディレクトリをモジュールとして認識させるには、ディレクトリ内に`(ディレクトリ名).er`ファイルを置く必要があります。
これはPythonの`__init__.py`と同じようなものですが、`__init__.py`と違ってディレクトリの外に置きます。

例として、以下のようなディレクトリ構成を考えてみましょう。

```console
└─┬ ./src
  ├─ app.er
  ├─ foo.er
  ├─ bar.er
  └─┬ bar
    ├─ baz.er
    └─ qux.er
```

`app.er`では`foo`モジュールと`bar`モジュールをインポートできます。`bar`ディレクトリがモジュールとして認識できるのは`bar.er`ファイルがあるためです。
`foo`モジュールはファイルからなるモジュールで、`bar`モジュールはディレクトリからなるモジュールです。`bar`モジュールはさらに`baz`, `qux`モジュールを内部に持ちます。
このモジュールは単に`bar`モジュールの属性であり、`app.er`からは以下のようにアクセスできます。

```python
# app.er
foo = import "foo"
bar = import "bar"
baz = bar.baz
# or `baz = import "bar/baz"`

main args =
    ...
```

サブモジュールにアクセスするための区切り文字が`/`であることに注意してください。これは、`bar.baz.er`のようなファイル名があり得るためです。
このようなファイル名は推奨されません。Ergでは`.er`のプレフィックスが意味を持つためです。
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

また、`.private.er`で終わるファイルはプライベートモジュールであり、同一ディレクトリのモジュールからしかアクセスできません。

```console
└─┬
  ├─ foo.er
  ├─ bar.er
  └─┬ bar
    ├─ baz.private.er
    └─ qux.er
```

```python
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

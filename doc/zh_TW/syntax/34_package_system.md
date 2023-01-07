# 包系統

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/34_package_system.md%26commit_hash%3De959b3e54bfa8cee4929743b0193a129e7525c61)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/34_package_system.md&commit_hash=e959b3e54bfa8cee4929743b0193a129e7525c61)

Erg包大致可以分為app包，即應用程序，以及lib包，即庫
應用包的入口點是`src/app.er`。`app.er` 中定義的`main` 函數被執行
lib 包的入口點是`src/lib.er`。導入包相當于導入 `lib.er`

一個包有一個稱為模塊的子結構，在 Erg 中是一個 Erg 文件或由 Erg 文件組成的目錄。外部 Erg 文件/目錄是作為模塊對象的可操作對象

為了將目錄識別為模塊，有必要在目錄中放置一個"(目錄名稱).er"文件
這類似于 Python 的 `__init__.py`，但與 `__init__.py` 不同的是，它放在目錄之外

例如，考慮以下目錄結構

```console
└─┬ ./src
  ├─ app.er
  ├─ foo.er
  ├─ bar.er
  └─┬ bar
    ├─ baz.er
    └─ qux.er
```

您可以在 `app.er` 中導入 `foo` 和 `bar` 模塊。由于 `bar.er` 文件，`bar` 目錄可以被識別為一個模塊
`foo` 模塊是由文件組成的模塊，`bar` 模塊是由目錄組成的模塊。`bar` 模塊還包含 `baz` 和 `qux` 模塊
該模塊只是 `bar` 模塊的一個屬性，可以從 `app.er` 訪問，如下所示

```python
# app.er
foo = import "foo"
bar = import "bar"
baz = bar.baz
# or `baz = import "bar/baz"`

main args =
    ...
```

請注意用于訪問子模塊的 `/` 分隔符。這是因為可以有諸如 `bar.baz.er` 之類的文件名
不鼓勵使用此類文件名，因為 `.er` 前綴在 Erg 中是有意義的
例如，用于測試的模塊。以 `.test.er` 結尾的文件是一個(白盒)測試模塊，它在運行測試時執行一個用 `@Test` 修飾的子例程

```console
└─┬ ./src
  ├─ app.er
  ├─ foo.er
  └─ foo.test.er
./src

```python
# app.er
foo = import "foo"

main args =
    ...
```

此外，以 .private.er 結尾的文件是私有模塊，只能由同一目錄中的模塊訪問

```console
└─┬
  ├─ foo.er
  ├─ bar.er
  └─┬ bar
    ├─ baz.private.er
    └─ qux.er
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
    <a href='./33_integration_with_Python.md'>上一頁</a> | <a href='./35_generator.md'>下一頁</a>
</p>

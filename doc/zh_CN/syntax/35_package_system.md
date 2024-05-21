# 包系统

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/35_package_system.md%26commit_hash%3Db80234b0663f57388f022b86f7c94a85b6250e9a)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/35_package_system.md&commit_hash=b80234b0663f57388f022b86f7c94a85b6250e9a)

Erg包大致可以分为app包，即应用程序，以及lib包，即库
应用包的入口点是`src/app.er`。`app.er` 中定义的`main` 函数被执行
lib 包的入口点是`src/lib.er`。导入包相当于导入 `lib.er`

一个包有一个称为模块的子结构，在 Erg 中是一个 Erg 文件或由 Erg 文件组成的目录。外部 Erg 文件/目录是作为模块对象的可操作对象

为了将目录识别为模块，有必要在目录中放置一个"(目录名称).er"文件
这类似于 Python 的 `__init__.py`，但与 `__init__.py` 不同的是，它放在目录之外

例如，考虑以下目录结构

```console
└─┬ ./src
  ├─ app.er
  ├─ foo.er
  ├─ bar.er
  └─┬ bar
    ├─ baz.er
    └─ qux.er
```

您可以在 `app.er` 中导入 `foo` 和 `bar` 模块。由于 `bar.er` 文件，`bar` 目录可以被识别为一个模块
`foo` 模块是由文件组成的模块，`bar` 模块是由目录组成的模块。`bar` 模块还包含 `baz` 和 `qux` 模块
该模块只是 `bar` 模块的一个属性，可以从 `app.er` 访问，如下所示

```python
# app.er
foo = import "foo"
bar = import "bar"
baz = bar.baz
# or `baz = import "bar/baz"`

main args =
    ...
```

请注意用于访问子模块的 `/` 分隔符。这是因为可以有诸如 `bar.baz.er` 之类的文件名
不鼓励使用此类文件名，因为 `.er` 前缀在 Erg 中是有意义的
例如，用于测试的模块。以 `.test.er` 结尾的文件是一个(白盒)测试模块，它在运行测试时执行一个用 `@Test` 修饰的子例程

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

此外，以 .private.er 结尾的文件是私有模块，只能由同一目录中的模块访问

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
    <a href='./34_integration_with_Python.md'>上一页</a> | <a href='./36_generator.md'>下一页</a>
</p>

# 包装系统

Erg 软件包大致可以分为 app 软件包（应用程序）和 lib 软件包（库）。app 包的入口点是。执行中定义的<gtr=“10”/>函数。lib 包的入口点是。导入包相当于导入<gtr=“12”/>。

软件包有一个称为模块的子结构。在 Erg 中，模块是 Erg 文件或由 Erg 文件组成的目录。外部 Erg 文件/目录可以作为模块对象进行操作。

要将目录识别为模块，必须在目录中放置文件。它类似于 Python 中的<gtr=“14”/>，但与<gtr=“15”/>不同，它位于目录之外。

例如，请考虑以下目录配置。


```console
└─┬ ./src
  ├─ app.er
  ├─ foo.er
  ├─ bar.er
  └─┬ bar
    ├─ baz.er
    └─ qux.er
```

允许你导入<gtr=“17”/>模块和<gtr=“18”/>模块。由于存在<gtr=“20”/>文件，<gtr=“19”/>目录可以识别为模块。<gtr=“21”/>模块是由文件组成的模块，<gtr=“22”/>模块是由目录组成的模块。<gtr=“23”/>模块还具有<gtr=“24”/>，<gtr=“25”/>模块。该模块仅是<gtr=“26”/>模块的属性，可通过<gtr=“27”/>访问。


```erg
# app.er
foo = import "foo"
bar = import "bar"
baz = bar.baz
# or `baz = import "bar/baz"`

main args =
    ...
```

请注意，用于访问子模块的分隔符为。这是因为文件名可能类似于<gtr=“29”/>。不建议使用这样的文件名。因为在 Erg 中，<gtr=“30”/>的前缀是有意义的。例如，测试模块。以<gtr=“31”/>结尾的文件是（白盒）测试模块，在执行测试时执行以<gtr=“32”/>装饰的子程序。


```console
└─┬ ./src
  ├─ app.er
  ├─ foo.er
  └─ foo.test.er
```


```erg
# app.er
foo = import "foo"

main args =
    ...
```

此外，以结尾的文件是专用模块，只能从同一目录中的模块访问。


```console
└─┬
  ├─ foo.er
  ├─ bar.er
  └─┬ bar
    ├─ baz.private.er
    └─ qux.er
```


```erg
# foo.er
bar = import "bar"
bar.qux
bar.baz # AttributeError: module 'baz' is private
```


```erg
# qux.er
baz = import "baz"
```

<p align='center'>
    <a href='./32_integration_with_Python.md'>Previous</a> | <a href='./34_generator.md'>Next</a>
</p>

# 包裝系統

Erg 軟件包大致可以分為 app 軟件包（應用程序）和 lib 軟件包（庫）。 app 包的入口點是。執行中定義的<gtr=“10”/>函數。 lib 包的入口點是。導入包相當於導入<gtr=“12”/>。

軟件包有一個稱為模塊的子結構。在 Erg 中，模塊是 Erg 文件或由 Erg 文件組成的目錄。外部 Erg 文件/目錄可以作為模塊對象進行操作。

要將目錄識別為模塊，必須在目錄中放置文件。它類似於 Python 中的<gtr=“14”/>，但與<gtr=“15”/>不同，它位於目錄之外。

例如，請考慮以下目錄配置。


```console
└─┬ ./src
  ├─ app.er
  ├─ foo.er
  ├─ bar.er
  └─┬ bar
    ├─ baz.er
    └─ qux.er
```

允許你導入<gtr=“17”/>模塊和<gtr=“18”/>模塊。由於存在<gtr=“20”/>文件，<gtr=“19”/>目錄可以識別為模塊。 <gtr=“21”/>模塊是由文件組成的模塊，<gtr=“22”/>模塊是由目錄組成的模塊。 <gtr=“23”/>模塊還具有<gtr=“24”/>，<gtr=“25”/>模塊。該模塊僅是<gtr=“26”/>模塊的屬性，可通過<gtr=“27”/>訪問。


```erg
# app.er
foo = import "foo"
bar = import "bar"
baz = bar.baz
# or `baz = import "bar/baz"`

main args =
    ...
```

請注意，用於訪問子模塊的分隔符為。這是因為文件名可能類似於<gtr=“29”/>。不建議使用這樣的文件名。因為在 Erg 中，<gtr=“30”/>的前綴是有意義的。例如，測試模塊。以<gtr=“31”/>結尾的文件是（白盒）測試模塊，在執行測試時執行以<gtr=“32”/>裝飾的子程序。


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

此外，以結尾的文件是專用模塊，只能從同一目錄中的模塊訪問。


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
# 模塊

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/24_module.md%26commit_hash%3D20aa4f02b994343ab9600317cebafa2b20676467)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/24_module.md&commit_hash=20aa4f02b994343ab9600317cebafa2b20676467)


Erg允許您將文件本身視為單個記錄(Record)。這稱為模塊

```python,checker_ignore
# foo.er
.i = 1
```

```python
# 定義 foo 模塊與定義這條記錄幾乎相同
foo = {.i = 1}
```

```python: bar.er
# bar.er
foo = import "foo"
print! foo # <module 'foo'>
assert foo.i == 1
```

由于模塊類型也是記錄類型，因此可以進行解構賦值

```python
{sin; cos;} = import "math"
```

## 模塊可見性

目錄和文件都可以是模塊
但是，在默認情況下，Erg不將目錄識別為Erg模塊。要讓它被識別，創建一個名為`__init__.er`的文件
`__init__.er`類似于Python中的`__init__.py`

```console
└─┬ bar
  └─ __init__.er
```

現在`bar`目錄被識別為一個模塊。如果`bar`中的唯一文件是`__init__.er`，則目錄結構沒有多大意義，但如果您想將多個模塊捆綁到一個模塊中，它會很有用。例如: 
```console
└─┬ bar
  ├─ __init__.er
  ├─ baz.er
  └─ qux.er
```

在`bar`目錄之外，您可以像下面這樣使用

```erg
bar = import "bar"
bar.baz.p!()
bar.qux.p!()
```

`__init__.er`不僅僅是一個將目錄作為模塊的標記，它還控制模塊的可見性

```erg
# __init__.er
# `. /` 指向當前目錄。可以省略
.baz = import ". /baz"
qux = import ". /qux"
.f x =
    .baz.f ...
.g x =
    qux.f ...
```

當你從外部導入 `bar` 模塊時，`baz` 模塊可以訪問，但 `qux` 模塊不能。

## 循環依賴

Erg 允許您定義模塊之間的循環依賴關系。

```python
# foo.er
bar = import "bar"
print! bar.g 1
.f x = x
```

```python
# bar.er
foo = import "foo"
print! foo.f 1
.g x = x
```

但是，由過程調用創建的變量不能在循環引用模塊中定義
這是因為 Erg 根據依賴關系重新排列定義的順序

```python
# foo.er
bar = import "bar"
print! bar.x
.x = g!(1) # 模塊錯誤：由過程調用創建的變量不能在循環引用模塊中定義
```

```python,checker_ignore
# bar.er
foo = import "foo"
print! foo.x
.x = 0
```

此外，作為入口點的 Erg 模塊（即 `__name__ == "__main__"` 的模塊）不能成為循環引用的主題

<p align='center'>
     <a href='./23_closure.md'>上一頁</a> | <a href='./25_object_system.md'>下一頁</a>
</p>
# 模块

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/24_module.md%26commit_hash%3Dfba8b193ce4270cb8c9236c4ed7bb8b2497af3fd)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/24_module.md&commit_hash=fba8b193ce4270cb8c9236c4ed7bb8b2497af3fd)


Erg允许您将文件本身视为单个记录(Record)。这称为模块

```python,checker_ignore
# foo.er
.i = 1
```

```python
# 定义 foo 模块与定义这条记录几乎相同
foo = {.i = 1}
```

```python: bar.er
# bar.er
foo = import "foo"
print! foo # <module 'foo'>
assert foo.i == 1
```

由于模块类型也是记录类型，因此可以进行解构赋值

```python
{sin; cos;} = import "math"
```

## 模块可见性

目录和文件都可以是模块
但是，在默认情况下，Erg不将目录识别为Erg模块。要让它被识别，创建一个名为`__init__.er`的文件
`__init__.er`类似于Python中的`__init__.py`

```console
└─┬ bar
  └─ __init__.er
```

现在`bar`目录被识别为一个模块。如果`bar`中的唯一文件是`__init__.er`，则目录结构没有多大意义，但如果您想将多个模块捆绑到一个模块中，它会很有用。例如: 
```console
└─┬ bar
  ├─ __init__.er
  ├─ baz.er
  └─ qux.er
```

在`bar`目录之外，您可以像下面这样使用

```erg
bar = import "bar"
bar.baz.p!()
bar.qux.p!()
```

`__init__.er`不仅仅是一个将目录作为模块的标记，它还控制模块的可见性

```erg
# __init__.er
# `. /` 指向当前目录。可以省略
.baz = import ". /baz"
qux = import ". /qux"
.f x =
    .baz.f ...
.g x =
    qux.f ...
```

当你从外部导入 `bar` 模块时，`baz` 模块可以访问，但 `qux` 模块不能。

## 循环依赖

Erg 允许您定义模块之间的循环依赖关系。

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

但是，由过程调用创建的变量不能在循环引用模块中定义
这是因为 Erg 根据依赖关系重新排列定义的顺序

```python
# foo.er
bar = import "bar"
print! bar.x
.x = g!(1) # 模块错误：由过程调用创建的变量不能在循环引用模块中定义
```

```python,checker_ignore
# bar.er
foo = import "foo"
print! foo.x
.x = 0
```

此外，作为入口点的 Erg 模块（即 `__name__ == "__main__"` 的模块）不能成为循环引用的主题

<p align='center'>
     <a href='./24_closure.md'>上一页</a> | <a href='./26_object_system.md'>下一页</a>
</p>
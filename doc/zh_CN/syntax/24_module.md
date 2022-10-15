# 模块

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/24_module.md%26commit_hash%3Db07c17708b9141bbce788d2e5b3ad4f365d342fa)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/24_module.md&commit_hash=b07c17708b9141bbce788d2e5b3ad4f365d342fa)

Erg允许您将文件本身视为单个记录(Record)。这称为模块。

```python: foo.er
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
{sin; cos; ...} = import "math"
```

## 模块可见性

```console
└─┬ ./src
   ├─ lib.er
   ├─ foo.er
   ├─bar.er
   └─┬ bar
     ├─ baz.er
     └─ qux.er
```

<p align='center'>
     <a href='./23_closure.md'>上一页</a> | <a href='./25_object_system.md'>下一页</a>
</p>
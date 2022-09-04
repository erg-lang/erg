# module

Erg allows you to think of the file itself as a single record. This is called a module.

```python: foo.er
# foo.er
.i = 1
```

```python
# 定义 foo 模块与定义这条记录几乎相同
foo = {.i = 1}
```

```python: bar.er
#bar.er
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
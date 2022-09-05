# module

Erg allows you to think of the file itself as a single record. This is called a module.

```python: foo.er
# foo.er
.i = 1
```

```python
# 定義 foo 模塊與定義這條記錄幾乎相同
foo = {.i = 1}
```

```python: bar.er
#bar.er
foo = import "foo"
print! foo # <module 'foo'>
assert foo.i == 1
```

由于模塊類型也是記錄類型，因此可以進行解構賦值

```python
{sin; cos; ...} = import "math"
```

## 模塊可見性

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
     <a href='./23_closure.md'>上一頁</a> | <a href='./25_object_system.md'>下一頁</a>
</p>
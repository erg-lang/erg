# 模块

Erg 可以将文件本身视为一条记录。我们称之为模块。


```erg: foo.er
# foo.er
.i = 1
```


```erg
# 定义 foo 模块和定义这条记录几乎一样
foo = {.i = 1}
```


```erg: bar.er
# bar.er
foo = import "foo"
print! foo # <module 'foo'>
assert foo.i == 1
```

模块化也是一种记录类型，因此可以进行分解赋值。


```erg
{sin; cos; ...} = import "math"
```

## 模块可见性


```console
└─┬ ./src
  ├─ lib.er
  ├─ foo.er
  ├─ bar.er
  └─┬ bar
    ├─ baz.er
    └─ qux.er
```

<p align='center'>
    <a href='./23_closure.md'>Previous</a> | <a href='./25_object_system.md'>Next</a>
</p>

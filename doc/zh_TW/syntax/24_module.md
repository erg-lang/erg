# 模塊

Erg 可以將文件本身視為一條記錄。我們稱之為模塊。


```erg: foo.er
# foo.er
.i = 1
```


```erg
# 定義 foo 模塊和定義這條記錄幾乎一樣
foo = {.i = 1}
```


```erg: bar.er
# bar.er
foo = import "foo"
print! foo # <module 'foo'>
assert foo.i == 1
```

模塊化也是一種記錄類型，因此可以進行分解賦值。


```erg
{sin; cos; ...} = import "math"
```

## 模塊可見性


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
# 展开分配

在分解赋值中，如果在变量之前放置，则所有剩余元素都可以扩展到该变量。这称为部署赋值。


```erg
[x, ...y] = [1, 2, 3]
assert x == 1
assert y == [2, 3]
x, ...y = (1, 2, 3)
assert x == 1
assert y == (2, 3)
```

## 提取分配

如果后没有写任何内容，则忽略其余元素并进行赋值。这种类型的展开赋值特别称为抽取赋值。提取赋值是一种有用的语法，用于将模块或记录中的特定属性本地化。


```erg
{sin; cos; tan; ..} = import "math"
```

然后，可以在本地使用。

记录也可以这样做。


```erg
record = {x = 1; y = 2}
{x; y; ...} = record
```

如果要全部展开，请使用。这就是 OCaml 等所说的<gtr=“9”/>。


```erg
record = {x = 1; y = 2}
{*} = record
assert x == 1 and y == 2
```

<p align='center'>
    <a href='./27_comprehension.md'>Previous</a> | <a href='./29_decorator.md'>Next</a>
</p>

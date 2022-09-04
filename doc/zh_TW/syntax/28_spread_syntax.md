# 展開分配

在分解賦值中，如果在變量之前放置，則所有剩餘元素都可以擴展到該變量。這稱為部署賦值。


```erg
[x, ...y] = [1, 2, 3]
assert x == 1
assert y == [2, 3]
x, ...y = (1, 2, 3)
assert x == 1
assert y == (2, 3)
```

## 提取分配

如果後沒有寫任何內容，則忽略其餘元素並進行賦值。這種類型的展開賦值特別稱為抽取賦值。提取賦值是一種有用的語法，用於將模塊或記錄中的特定屬性本地化。


```erg
{sin; cos; tan; ..} = import "math"
```

然後，可以在本地使用。

記錄也可以這樣做。


```erg
record = {x = 1; y = 2}
{x; y; ...} = record
```

如果要全部展開，請使用。這就是 OCaml 等所說的<gtr=“9”/>。


```erg
record = {x = 1; y = 2}
{*} = record
assert x == 1 and y == 2
```

<p align='center'>
    <a href='./27_comprehension.md'>Previous</a> | <a href='./29_decorator.md'>Next</a>
</p>
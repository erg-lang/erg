# 下標

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/container_ownership.md%26commit_hash%3D20aa4f02b994343ab9600317cebafa2b20676467)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/container_ownership.md&commit_hash=20aa4f02b994343ab9600317cebafa2b20676467)

`[]` 不同于普通的方法

```python
a = [!1, !2]
a[0].inc!()
assert a == [2, 2]
```

回想一下，子例程的返回值不能是引用
這里的 `a[0]` 的類型顯然應該是 `Ref!(Int!)`(`a[0]` 的類型取決于上下文)
所以 `[]` 實際上是特殊語法的一部分，就像 `.` 一樣。與 Python 不同，它不能被重載
也無法在方法中重現 `[]` 的行為

```python
C = Class {i = Int!}
C.steal(self) =
    self::i
```

```python,compile_fail
C. get(ref self) =
    self::i # 類型錯誤:`self::i`是`Int!`(需要所有權)但`get`不擁有`self`
```

```python
# OK (分配)
c = C.new({i = 1})
i = c.steal()
i.inc!()
assert i == 2
# or (own_do!)
own_do! C.new({i = 1}).steal(), i => i.inc!()
```

```python
# NG
C.new({i = 1}).steal().inc!() # OwnershipWarning: `C.new({i = 1}).steal()` is not owned by anyone
# hint: assign to a variable or use `uwn_do!`
```

此外，`[]` 可以不承認，但元素不會移動

```python
a = [!1, !2]
i = a[0]
i.inc!()
assert a[1] == 2
a[0] # 所有權錯誤:`a[0]`被移動到`i`
```

<p align='center'>
    <a href='./11_dict.md'>上一頁</a> | <a href='./13_tuple.md'>下一頁</a>
</p>

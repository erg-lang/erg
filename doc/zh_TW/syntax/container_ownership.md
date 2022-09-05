# 下標(索引訪問)

`[]` 不同于普通的方法。

```python
a = [!1, !2]
a[0].inc!()
assert a == [2, 2]
```

回想一下，子例程的返回值不能是引用。
這里的 `a[0]` 的類型顯然應該是 `Ref!(Int!)`(`a[0]` 的類型取決于上下文)。
所以 `[]` 實際上是特殊語法的一部分，就像 `.` 一樣。 與 Python 不同，它不能被重載。
也無法在方法中重現 `[]` 的行為。

```python
C = Class {i = Int!}
C. get(ref self) =
    self::i # 類型錯誤：`self::i` 是 `Int!`(需要所有權)但 `get` 不擁有 `self`
C.steal(self) =
    self::i
#NG
C.new({i = 1}).steal().inc!() # 所有權警告：`C.new({i = 1}).steal()` 不屬于任何人
# 提示：分配給變量或使用 `uwn_do!`
# OK (分配)
c = C.new({i = 1})
i = c.steal()
i.inc!()
assert i == 2
# or (own_do!)
own_do! C.new({i = 1}).steal(), i => i.inc!()
```

此外，`[]` 可以不承認，但元素不會移動

```python
a = [!1, !2]
i = a[0]
i.inc!()
assert a[1] == 2
a[0] # 所有權錯誤：`a[0]` 被移動到 `i`
```
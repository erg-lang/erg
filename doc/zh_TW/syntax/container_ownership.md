# 下标(索引访问)

`[]` 不同于普通的方法。

```python
a = [!1, !2]
a[0].inc!()
assert a == [2, 2]
```

回想一下，子例程的返回值不能是引用。
这里的 `a[0]` 的类型显然应该是 `Ref!(Int!)`(`a[0]` 的类型取决于上下文)。
所以 `[]` 实际上是特殊语法的一部分，就像 `.` 一样。 与 Python 不同，它不能被重载。
也无法在方法中重现 `[]` 的行为。

```python
C = Class {i = Int!}
C. get(ref self) =
    self::i # 类型错误：`self::i` 是 `Int!`(需要所有权)但 `get` 不拥有 `self`
C.steal(self) =
    self::i
#NG
C.new({i = 1}).steal().inc!() # 所有权警告：`C.new({i = 1}).steal()` 不属于任何人
# 提示：分配给变量或使用 `uwn_do!`
# OK (分配)
c = C.new({i = 1})
i = c.steal()
i.inc!()
assert i == 2
# or (own_do!)
own_do! C.new({i = 1}).steal(), i => i.inc!()
```

此外，`[]` 可以不承认，但元素不会移动

```python
a = [!1, !2]
i = a[0]
i.inc!()
assert a[1] == 2
a[0] # 所有权错误：`a[0]` 被移动到 `i`
```
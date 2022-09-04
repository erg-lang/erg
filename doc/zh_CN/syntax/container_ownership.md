# 下标（索引访问）

`[]` 不同于普通的方法。

```python
a = [!1, !2]
a[0].inc!()
assert a == [2, 2]
```

回想一下，子例程的返回值不能是引用。
这里的 `a[0]` 的类型显然应该是 `Ref!(Int!)`（`a[0]` 的类型取决于上下文）。
所以 `[]` 实际上是特殊语法的一部分，就像 `.` 一样。 与 Python 不同，它不能被重载。
也无法在方法中重现 `[]` 的行为。

```python
C = Class {i = Int!}
C. get(ref self) =
    self::i # TypeError: `self::i` is `Int!` (require ownership) but `get` doesn't own `self`
C.steal(self) =
    self::i
#NG
C.new({i = 1}).steal().inc!() # OwnershipWarning: `C.new({i = 1}).steal()` is not owned by anyone
# hint: assign to a variable or use `uwn_do!`
# OK (assigning)
c = C.new({i = 1})
i = c.steal()
i.inc!()
assert i == 2
# or (own_do!)
own_do! C.new({i = 1}).steal(), i => i.inc!()
```

Also, `[]` can be disowned, but the element is not shifted.

```python
a = [!1, !2]
i = a[0]
i.inc!()
assert a[1] == 2
a[0] # OwnershipError: `a[0]` is moved to `i`
```
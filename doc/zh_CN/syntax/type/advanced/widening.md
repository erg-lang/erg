# 类型加宽

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/advanced/widening.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced/widening.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

例如，定义多相关系数如下。

```python
ids|T|(x: T, y: T) = x, y
```

分配同一类的一对实例并没有错。
当您分配另一个具有包含关系的类的实例对时，它会向上转换为较大的类并成为相同的类型。
另外，很容易理解，如果分配了另一个不在包含关系中的类，就会发生错误。

```python
assert ids(1, 2) == (1, 2)
assert ids(1, 2.0) == (1.0, 2.0)
ids(1, "a") #TypeError
```

现在，具有不同派生类型的类型呢?

```python
i: Int or Str
j: Int or NoneType
ids(i, j) # ?
```

在解释这一点之前，我们必须关注 Erg 的类型系统实际上并不关注(运行时)类这一事实。

```python
1: {__valueclass_tag__ = Phantom Int}
2: {__valueclass_tag__ = Phantom Int}
2.0: {__valueclass_tag__ = Phantom Ratio}
"a": {__valueclass_tag__ = Phantom Str}
ids(1, 2): {__valueclass_tag__ = Phantom Int} and {__valueclass_tag__ = Phantom Int} == {__valueclass_tag__ = Phantom Int}
ids(1, 2.0): {__valueclass_tag__ = Phantom Int} and {__valueclass_tag__ = Phantom Ratio} == {__valueclass_tag__ = Phantom Ratio} # Int < Ratio
ids(1, "a"): {__valueclass_tag__ = Phantom Int} and {__valueclass_tag__ = Phantom Str} == Never # 类型错误
```

我看不到该类，因为它可能无法准确看到，因为在 Erg 中，对象的类属于运行时信息。
例如，一个`Int`或Str`类型的对象的类是`Int`或`Str`，但你只有通过执行才能知道它是哪一个。
当然，`Int` 类型的对象的类被定义为 `Int`，但是在这种情况下，从类型系统中可见的是 `Int` 的结构类型 `{__valueclass_tag__ = Int}`。

现在让我们回到另一个结构化类型示例。 总之，上述代码将导致类型错误，因为类型不匹配。
但是，如果您使用类型注释进行类型扩展，编译将通过。

```python
i: Int or Str
j: Int or NoneType
ids(i, j) # 类型错误：i 和 j 的类型不匹配
# 提示：尝试扩大类型(例如 ids<Int or Str or NoneType>)
ids<Int or Str or NoneType>(i, j) # OK
```

`A 和 B` 有以下可能性。

* `A and B == A`：当`A <: B`或`A == B`时。
* `A and B == B`：当 `A :> B` 或 `A == B` 时。
* `A and B == {}`：当 `!(A :> B)` 和 `!(A <: B)` 时。

`A 或 B` 具有以下可能性。

* `A 或 B == A`：当`A :> B` 或`A == B` 时。
* `A or B == B`：当`A <: B`或`A == B`时。
* `A 或 B` 是不可约的(独立类型)：如果 `!(A :> B)` 和 `!(A <: B)`。

## 子程序定义中的类型扩展

如果返回类型不匹配，Erg 默认会出错。

```python
parse_to_int s: Str =
    if not s.is_numeric():
        do parse_to_int::return error("not numeric")
... # 返回 Int 对象
# 类型错误：返回值类型不匹配
# 3 | 做 parse_to_int::return error("not numeric")
# └─ Error
# 4 | ...
# └ Int
```

为了解决这个问题，需要将返回类型显式指定为 Or 类型

```python
parse_to_int(s: Str): Int or Error =
    if not s.is_numeric():
        do parse_to_int::return error("not numeric")
    ... # 返回 Int 对象
```

这是设计使然，这样您就不会无意中将子例程的返回类型与另一种类型混合。
但是，如果返回值类型选项是具有包含关系的类型，例如 `Int` 或 `Nat`，它将与较大的对齐。
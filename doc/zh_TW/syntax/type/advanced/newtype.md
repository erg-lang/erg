# 新类型模式

这是 Rust 中常用的 newtype 模式的 Erg 版本。

Erg 允许定义类型别名如下，但它们只引用相同的类型。

```python
UserID = Int
```

因此，例如，如果你有一个规范，类型为 `UserId` 的数字必须是一个正的 8 位数字，你可以输入 `10` 或 `-1`，因为它与类型 `Int` 相同 . 如果设置为 `Nat`，则可以拒绝 `-1`，但 8 位数字的性质不能仅用 Erg 的类型系统来表达。

此外，例如，在设计数据库系统时，假设有几种类型的 ID：用户 ID、产品 ID、产品 ID 和用户 ID。 如果 ID 类型的数量增加，例如用户 ID、产品 ID、订单 ID 等，可能会出现将不同类型的 ID 传递给不同函数的 bug。 即使用户 ID 和产品 ID 在结构上相同，但它们在语义上是不同的。

对于这种情况，newtype 模式是一个很好的设计模式。

```python
UserId = Class {id = Nat}
UserId.
    new id: Nat =
        assert id.dights().len() == 8, else: "UserId 必须是长度为 8 的正数"
        UserId::__new__ {id;}

i = UserId.new(10000000)
print! i # <__main__.UserId object>
i + UserId.new(10000001) # TypeError: + is not implemented between `UserId` and `UserId
```

构造函数保证 8 位数字的前置条件。
`UserId` 失去了 `Nat` 拥有的所有方法，所以每次都必须重新定义必要的操作。
如果重新定义的成本不值得，最好使用继承。 另一方面，在某些情况下，方法丢失是可取的，因此请根据情况选择适当的方法。

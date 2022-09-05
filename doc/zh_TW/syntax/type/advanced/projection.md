# 投影类型

投影类型表示如下代码中的“Self.AddO”等类型。

```python
Add R = Trait {
    . `_+_` = Self, R -> Self.AddO
    .AddO = Type
}

AddForInt = Patch(Int, Impl := Add Int)
AddForInt.
    AddO = Int
```

类型“Add(R)”可以说是定义了与某个对象的加法的类型。 由于方法应该是一个类型属性，`+` 类型声明应该写在缩进下面。
`Add` 类型的场景是声明 `.AddO = Type`，而 `.AddO` 类型的实体是一个投影类型，由一个作为 ` 子类型的类型持有 添加`。 例如，`Int.AddO = Int`、`Odd.AddO = Even`。

```python
assert Int < Add
assert Int.AddO == Int
assert Odd < Add
assert Odd.AddO == Even
```

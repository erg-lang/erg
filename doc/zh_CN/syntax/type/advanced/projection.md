# 投影类型

投影类型表示以下代码中类似于的类型。


```erg
Add R = Trait {
    .`_+_` = Self, R -> Self.AddO
    .AddO = Type
}

AddForInt = Patch(Int, Impl := Add Int)
AddForInt.
    AddO = Int
```

类型定义了与某个对象的相加。由于方法应该是类型属性，因此<gtr=“5”/>的类型声明必须位于缩进下面。<gtr=“6”/>类型的 misso 是<gtr=“7”/>声明，其投影类型<gtr=“8”/>类型的实体具有属于<gtr=“9”/>的子类型的类型。例如，<gtr=“10”/>，<gtr=“11”/>。


```erg
assert Int < Add
assert Int.AddO == Int
assert Odd < Add
assert Odd.AddO == Even
```

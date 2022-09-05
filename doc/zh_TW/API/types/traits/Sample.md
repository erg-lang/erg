# Sample

具有“随机”选择实例的`sample`和`sample!`方法的特征。`sample`方法总是返回相同的实例，而`sample!`方法返回一个随机实例，该实例随调用而变化

请注意，这是一个假设您想要一个适当的实例进行测试等的特征，并且它不一定是随机的。 如果您想要随机抽样，请使用“随机”模块。

所有主要的值类都实现了 `Sample`。它还在由“Sample”类组成的元组类型、记录类型、Or类型和筛选类型中实现

```python
assert Int.sample() == 42
assert Str.sample() == "example"
# Int默认在64bit范围内采样
print! Int.sample!() # 1313798
print! {x = Int; y = Int}.sample!() # {x = -32432892, y = 78458576891}
```

下面是一个`Sample`的实现示例

```python
EmailAddress = Class {header = Str; domain = Str}, Impl=Sample and Show
@Impl Show
EmailAddress.
    show self = "{self::header}@{self::domain}"
@Impl Sample
EmailAddress.
    sample(): Self = Self.new "sample@gmail.com"
    sample!(): Self =
        domain = ["gmail.com", "icloud.com", "yahoo.com", "outlook.com", ...].sample!()
        header = AsciiStr.sample!()
        Self.new {header; domain}
```

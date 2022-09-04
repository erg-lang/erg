# Sample

具有“隨機”選擇實例的`sample`和`sample!`方法的特徵。 `sample`方法總是返回相同的實例，而`sample!`方法返回一個隨機實例，該實例隨調用而變化

請注意，這是一個假設您想要一個適當的實例進行測試等的特徵，並且它不一定是隨機的。如果您想要隨機抽樣，請使用“隨機”模塊。

所有主要的值類都實現了 `Sample`。它還在由“Sample”類組成的元組類型、記錄類型、Or類型和篩選類型中實現

```erg
assert Int.sample() == 42
assert Str.sample() == "example"
# Int默認在64bit範圍內採樣
print! Int.sample!() # 1313798
print! {x = Int; y = Int}.sample!() # {x = -32432892, y = 78458576891}
```

下面是一個`Sample`的實現示例

```erg
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
# 投影類型

投影類型表示以下代碼中類似於的類型。


```erg
Add R = Trait {
    .`_+_` = Self, R -> Self.AddO
    .AddO = Type
}

AddForInt = Patch(Int, Impl := Add Int)
AddForInt.
    AddO = Int
```

類型定義了與某個對象的相加。由於方法應該是類型屬性，因此<gtr=“5”/>的類型聲明必須位於縮進下面。 <gtr=“6”/>類型的 misso 是<gtr=“7”/>聲明，其投影類型<gtr=“8”/>類型的實體具有屬於<gtr=“9”/>的子類型的類型。例如，<gtr=“10”/>，<gtr=“11”/>。


```erg
assert Int < Add
assert Int.AddO == Int
assert Odd < Add
assert Odd.AddO == Even
```
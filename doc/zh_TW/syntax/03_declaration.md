# 聲明

聲明是指定要使用的變量類型的語法。可以在代碼中的任何地方聲明，但不能只聲明變量。必須始終初始化。賦值後聲明可以檢查類型是否與賦值對象匹配。


```erg
i: Int
# i: Int = 2可以與賦值同時聲明
i = 2
i: Num
i: Nat
i: -2..2
i: {2}
```

賦值後聲明類似於類型檢查，但在編譯時進行檢查。運行時使用<gtr=“6”/>進行類型檢查可以用“可能是 XX”進行檢查，但編譯時使用<gtr=“7”/>進行類型檢查是嚴格的。如果沒有確定是“某某型”，就無法通過檢查，就會出現錯誤。


```erg
i = (-1..10).sample!()
assert i in Nat # 這可能會通過
i: Int # 這通過了
i: Nat # 這不起作用（因為 -1 不是 Nat 的元素）
```

可以通過兩種方式聲明函數。


```erg
f: (x: Int, y: Int) -> Int
f: (Int, Int) -> Int
```

如果顯式聲明參數名稱，則在定義時如果名稱不同，將導致類型錯誤。如果你想給出參數名稱的任意性，可以使用第二種方法聲明它。在這種情況下，類型檢查只顯示方法名稱及其類型。


```erg
T = Trait {
    .f = (x: Int, y: Int): Int
}

C = Class(U, Impl := T)
C.f(a: Int, b: Int): Int = ... # TypeError: `.f` must be type of `(x: Int, y: Int) -> Int`, not `(a: Int, b: Int) -> Int`
```

<p align='center'>
    <a href='./02_name.md'>Previous</a> | <a href='./04_function.md'>Next</a>
</p>
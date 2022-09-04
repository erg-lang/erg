# 封閉

Erg 子例程具有一個名為“閉包”的功能，用於捕獲外部變量。


```erg
outer = 1
f x = outer + x
assert f(1) == 2
```

可以捕捉可變對象，也可以捕捉不變對象。


```erg
sum = !0
for! 1..10, i =>
    sum.add! i
assert sum == 45

p! x =
    sum.add! x
p!(1)
assert sum == 46
```

但需要注意的是，函數無法捕獲可變對象。如果可以在函數中引用可變對象，則可以編寫如下所示的代碼。


```erg
# !!! 這個代碼實際上給出了一個錯誤!!!
i = !0
f x = i + x
assert f 1 == 1
i.add! 1
assert f 1 == 2
```

函數應該為相同的參數返回相同的值，但假設已被破壞。請注意，是在調用時首次計算的。

如果需要函數定義時可變對象的內容，則調用。


```erg
i = !0
immut_i = i.clone().freeze()
f x = immut_i + x
assert f 1 == 1
i.add! 1
assert f 1 == 1
```

## 避免可變狀態，函數編程


```erg
# Erg
sum = !0
for! 1..10, i =>
    sum.add! i
assert sum == 45
```

在 Python 中，可以按如下方式編寫上面的等效程序。


```python
# Python
sum = 0
for i in range(1, 10):
    sum += i
assert sum == 45
```

但 Erg 建議使用更簡單的寫法。使用局部化使用函數的狀態的樣式，而不是使用子例程和可變對象來維護狀態。這稱為函數型編程。


```erg
# Functional style
sum = (1..10).sum()
assert sum == 45
```

上面的代碼與剛才的結果完全相同，但我們可以看到它要簡單得多。

除了求和之外，還可以使用函數執行更多操作。 <gtr=“12”/>是迭代器方法，它為每個小版本執行參數<gtr=“13”/>。存儲結果的計數器的初始值由<gtr=“14”/>指定，然後存儲在<gtr=“15”/>中。


```erg
# start with 0, result will
sum = (1..10).fold(init: 0, f: (acc, i) -> acc + i)
assert sum == 45
```

Erg 的設計是為了使用不變的對象進行編程，從而提供自然簡潔的描述。

<p align='center'>
    <a href='./22_subroutine.md'>Previous</a> | <a href='./24_module.md'>Next</a>
</p>
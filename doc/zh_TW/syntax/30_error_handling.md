# 錯誤處理系統

主要使用 Result 類型。在 Erg 中，如果丟棄 Error 類型的對象（不在頂層），則會發生錯誤。

## 異常，與 Python 的互操作

Erg 沒有異常機制（Exception）。導入 Python 函數時

* 返回類型
* 類型（可能導致運行時錯誤）

的兩個選項，在中默認為後者。如果要作為前者導入，請在<gtr=“9”/>的<gtr=“10”/>中指定<gtr=“11”/>（<gtr=“12”/>）。

## 異常和結果類型

類型表示可能出現錯誤的值。使用<gtr=“14”/>處理錯誤在某些方面優於異常機制。首先，從類型定義可以看出子程序可能會出錯，在實際使用時也一目了然。


```python
# Python
try:
    x = foo().bar()
    y = baz()
    qux()
except e:
    print(e)
```

在上面的示例中，僅此代碼並不知道異常是從哪個函數調度的。即使追溯到函數定義，也很難確定該函數是否會出現異常。


```erg
# Erg
try!:
    do!:
        x = foo!()?.bar()
        y = baz!()
        qux!()?
    e =>
        print! e
```

相反，在本示例中，和<gtr=“16”/>可以生成錯誤。確切地說，<gtr=“17”/>也可能是<gtr=“18”/>類型，但在使用中值時，你必須執行此操作。

使用類型的好處遠不止這些。類型也是線程安全的。這意味著錯誤信息可以在並行執行期間（很容易）傳遞。

## Context

/<gtr=“22”/>類型不會產生副作用，因此它不具有與異常不同的諸如發送位置之類的信息（上下文），但可以使用<gtr=“23”/>方法將信息添加到<gtr=“24”/>對象。 <gtr=“25”/>方法是使用<gtr=“26”/>對象本身來創建新的<gtr=“27”/>對象的方法。它是可鏈接的，可以有多個上下文。


```erg
f() =
    todo() \
        .context "to be implemented in ver 1.2" \
        .context "and more hints ..."

f()
# Error: not implemented yet
# hint: to be implemented in ver 1.2
# hint: and more hints ...
```

注意，屬性（如<gtr=“28”/>和<gtr=“29”/>）不是次要屬性，因此不是 context，不能覆蓋最初生成的屬性。

## 棧跟踪

類型由於其方便性，在其他語言中也被廣泛採用，但與異常機制相比，其缺點是錯誤的來源變得更難理解。因此，在 Erg 中，使<gtr=“32”/>對象具有<gtr=“33”/>屬性，模擬地再現了異常機制那樣的棧跟踪。 <gtr=“34”/>是調用對象的數組。每當 Error 對象<gtr=“35”/>（包括<gtr=“36”/>所致）時，它的調用子例程將加載到<gtr=“37”/>中。如果<gtr=“38”/>在環境中<gtr=“39”/>或<gtr=“40”/>，它將死機並顯示回溯。


```erg
f x =
    ...
    y = foo.try_some(x)?
    ...

g x =
    y = f(x)?
    ...

i = g(1)?
# Traceback (most recent call first):
#    ...
#    Foo.try_some, line 10, file "foo.er"
#    10 | y = foo.try_some(x)?
#    module::f, line 23, file "foo.er"
#    23 | y = f(x)?
#    module::g, line 40, file "foo.er"
#    40 | i = g(1)?
# Error: ...
```

## 恐慌

Erg 還存在一個名為的機制來處理不可恢復的錯誤。不可恢復的錯誤可能是由外部因素引起的錯誤，例如軟/硬件故障，致命到無法繼續執行代碼的程度，或者是程序編寫者不想要的錯誤。如果發生這種情況，由於程序員的努力無法使其恢復正常系統，因此當場終止程序。這叫做“恐慌”。

使用函數執行死機。


```erg
panic "something went wrong!"
```

<p align='center'>
    <a href='./29_decorator.md'>Previous</a> | <a href='./31_pipeline.md'>Next</a>
</p>
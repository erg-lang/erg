# 類型擴展（Type Widening）

例如定義如下的多相關數。


```erg
ids|T|(x: T, y: T) = x, y
```

代入相同類的實例對沒有任何問題。如果代入包含關係中的其他類的實例對的話，就會被上播到較大的一方，成為相同的類型。另外，如果代入不在包含關係中的其他類，就會出現錯誤，這也很容易理解。


```erg
assert ids(1, 2) == (1, 2)
assert ids(1, 2.0) == (1.0, 2.0)
ids(1, "a") # TypeError
```

那麼，擁有其他結構型的型的情況又會怎樣呢？


```erg
i: Int or Str
j: Int or NoneType
ids(i, j) # ?
```

在解釋這一點之前，我們必須注意一個事實，即 Erg 類型系統實際上沒有看到類（在運行時）。


```erg
1: {__valueclass_tag__ = Phantom Int}
2: {__valueclass_tag__ = Phantom Int}
2.0: {__valueclass_tag__ = Phantom Ratio}
"a": {__valueclass_tag__ = Phantom Str}
ids(1, 2): {__valueclass_tag__ = Phantom Int} and {__valueclass_tag__ = Phantom Int} == {__valueclass_tag__ = Phantom Int}
ids(1, 2.0): {__valueclass_tag__ = Phantom Int} and {__valueclass_tag__ = Phantom Ratio} == {__valueclass_tag__ = Phantom Ratio} # Int < Ratio
ids(1, "a"): {__valueclass_tag__ = Phantom Int} and {__valueclass_tag__ = Phantom Str} == Never # TypeError
```

之所以沒有看到類，是因為有時不能正確看到，這是因為在 Erg 中對象的類屬於運行時信息。例如，型對象的類是<gtr=“9”/>或者<gtr=“10”/>，這是哪一個只有執行後才能知道。當然，<gtr=“11”/>型的對象的類是由<gtr=“12”/>確定的，這時從類型系統中也能看到<gtr=“13”/>的結構型<gtr=“14”/>。

現在，讓我們回到另一個結構類型的例子。從結論上來說，上面的代碼如果沒有類型，就會成為 TypeError。但是，如果用類型註釋進行類型擴大，編譯就可以通過。


```erg
i: Int or Str
j: Int or NoneType
ids(i, j) # TypeError: types of i and j not matched
# hint: try type widening (e.g. ids<Int or Str or NoneType>)
ids<Int or Str or NoneType>(i, j) # OK
```

有以下可能性。

* ：<gtr=“17”/>或<gtr=“18”/>。
* ：<gtr=“20”/>或<gtr=“21”/>時。
* ：<gtr=“23”/>且<gtr=“24”/>時。

有以下可能性。

* ：<gtr=“27”/>或<gtr=“28”/>時。
* ：<gtr=“30”/>或<gtr=“31”/>。
* 不能簡化（獨立類型）：當<gtr=“33”/>且<gtr=“34”/>時。

## 子例程定義中的類型擴展

在 Erg 中，返回值類型不一致時默認為錯誤。


```erg
parse_to_int s: Str =
    if not s.is_numeric():
        do parse_to_int::return error("not numeric")
    ... # return Int object
# TypeError: mismatch types of return values
#     3 | do parse_to_int::return error("not numeric")
#                                 └─ Error
#     4 | ...
#         └ Int
```

為了解決這一問題，必須將返回類型顯式指定為 Or 類型。


```erg
parse_to_int(s: Str): Int or Error =
    if not s.is_numeric():
        do parse_to_int::return error("not numeric")
    ... # return Int object
```

這是為了不讓子程序的返回值類型無意中混入其他類型的設計。但是，當返回值類型的選項是或<gtr=“36”/>等具有包含關係的類型時，向較大的類型對齊。
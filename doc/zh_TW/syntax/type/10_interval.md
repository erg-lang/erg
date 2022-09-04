# Interval Type

對象最基本的用法是用作迭代器。


```erg
for! 0..9, i =>
    print! i
```

請注意，與 Python 不同，最後一個數字是包含的。

但是，這並不是對象的唯一用途。也可以作為模具使用。這些類型稱為“區間類型”（Interval type）。


```erg
i: 0..10 = 2
```

類型與<gtr=“8”/>等效，<gtr=“9”/>和<gtr=“10”/>類型與<gtr=“11”/>等效。 <gtr=“12”/>也可以寫成<gtr=“13”/>。 <gtr=“14”/>表示類型<gtr=“15”/>的任何實例。

也可以用作迭代器，因此可以按相反的順序指定，如，但不能反轉<gtr=“17”/>，<gtr=“18”/>和<gtr=“19”/>的方向。


```erg
a = 0..10 # OK
b = 0..<10 # OK
c = 10..0 # OK
d = 10<..0 # Syntax error
e = 10..<0 # Syntax error
f = 10<..<0 # Syntax error
```

範圍運算符（range operator）也可以用於非數字類型，只要它們是不變的。


```erg
Alphabet = "A".."z"
```
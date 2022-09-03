# Interval Type

对象最基本的用法是用作迭代器。


```erg
for! 0..9, i =>
    print! i
```

请注意，与 Python 不同，最后一个数字是包含的。

但是，这并不是对象的唯一用途。也可以作为模具使用。这些类型称为“区间类型”（Interval type）。


```erg
i: 0..10 = 2
```

类型与<gtr=“8”/>等效，<gtr=“9”/>和<gtr=“10”/>类型与<gtr=“11”/>等效。<gtr=“12”/>也可以写成<gtr=“13”/>。<gtr=“14”/>表示类型<gtr=“15”/>的任何实例。

也可以用作迭代器，因此可以按相反的顺序指定，如，但不能反转<gtr=“17”/>，<gtr=“18”/>和<gtr=“19”/>的方向。


```erg
a = 0..10 # OK
b = 0..<10 # OK
c = 10..0 # OK
d = 10<..0 # Syntax error
e = 10..<0 # Syntax error
f = 10<..<0 # Syntax error
```

范围运算符（range operator）也可以用于非数字类型，只要它们是不变的。


```erg
Alphabet = "A".."z"
```

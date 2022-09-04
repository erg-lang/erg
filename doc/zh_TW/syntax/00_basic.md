# 基本信息

> ：此文檔尚未完成。未進行校樣（文體、正確鏈接等）。此外，Erg 的語法在 0.* 版本之間可能會有顛覆性的改變，隨之而來的文檔更新可能跟不上。請事先諒解。
> 此外，如果你發現本文檔中的錯誤，請從或<gtr=“11”/>提出更正建議。

本文檔介紹了 Erg 的基本語法。和<gtr=“13”/>位於不同的目錄中。

## Hello, World!

首先按照慣例舉辦 Hello World 活動吧。


```erg
print!("Hello, World!")
```

跟 Python 和同系語言差不多。引人注目的是後面的<gtr=“15”/>，我會慢慢解釋它的含義。此外，在 Erg 中，如果解釋不准確，可以省略括號<gtr=“16”/>。與 Ruby 類似，它可以省略括號，但它不能具有多個解釋，也不能在參數為 0 時省略<gtr=“17”/>，就像 Python 一樣。


```erg
print! "Hello, World!" # OK
print! "Hello,", "World!" # OK
print!() # OK
print! # OK, but this does not mean to call, simply to get `print!` as a callable object

print! f x # OK, interpreted as `print!(f(x))`
print!(f(x, y)) # OK
print! f(x, y) # OK
print! f(x, g y) # OK
print! f x, y # NG, can be taken to mean either `print!(f(x), y)` or `print!(f(x, y))`
print!(f x, y) # NG, can be taken to mean either `print!(f(x), y)` or `print!(f(x, y))`
print! f(x, g y, z) # NG, can be taken to mean either `print!(x, g(y), z)` or `print!(x, g(y, z))`
```

## 腳本

Erg 代碼稱為腳本。可以以文件格式（.er）保存和運行腳本。

## 註釋

及更高版本將作為註釋忽略。當你想要解釋代碼的意圖，或者想要暫時禁用代碼時，可以使用此選項。


```erg
# コメント
## `#`以降は改行されるまで無視されるので、`#`は何個あってもOK
#[
複數行コメント
対応する`]#`のところまでずっとコメントとして扱われます
]#
```

## 表達式，分隔符

腳本是一系列表達式（expression）。表達式是一個可以計算和評估的東西，在 Erg 中幾乎所有的東西都是表達式。使用分隔符-換行符或分號-分隔每個表達式。 Erg 腳本基本上是從左到右、從上到下進行評估的。


```erg
n = 1 # 代入式
f(1, 2) # 関數適用式
1 + 1 # 演算子適用式
f(1, 2); 1 + 1
```

有一個稱為即時塊的功能，它使用塊中最後計算的表達式作為變量的值，如下所示。這與無參數函數不同，它不使用。請注意，方塊只在現場評估一次。


```erg
i =
    x = 1
    x + 1
assert i == 2
```

這不能通過分號（）來實現。


```erg
i = (x = 1; x + 1) # SyntaxError: cannot use `;` in parentheses
```

## 縮進

Erg 使用與 Python 相同的縮進來表示塊。觸發塊開始的運算符（特殊格式）有五種：，<gtr=“23”/>，<gtr=“24”/>，<gtr=“25”/>和<gtr=“26”/>（其他運算符不是，但<gtr=“27”/>和<gtr=“28”/>也會生成縮進）。它們各自的含義將在後面介紹。


```erg
f x, y =
    x + y

for! 0..9, i =>
    print! i

for! 0..9, i =>
    print! i; print! i

ans = match x:
    0 -> "zero"
    _: 0..9 -> "1 dight"
    _: 10..99 -> "2 dights"
    _ -> "unknown"
```

如果一行太長，可以使用在中間換行。


```erg
# this does not means `x + y + z` but means `x; +y; +z`
x
+ y
+ z

# this means `x + y + z`
x \
+ y \
+ z
```

<p align='center'>
    Previous | <a href='./01_literal.md'>Next</a>
</p>
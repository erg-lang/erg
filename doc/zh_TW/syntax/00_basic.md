# 基本

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/00_basic.md%26commit_hash%3D736dcb272d2132883ec7b883f7694829398be61e)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/00_basic.md&commit_hash=736dcb272d2132883ec7b883f7694829398be61e)

> __Warning__: 本文檔不完整。它未經校對(樣式、正確鏈接、誤譯等)。此外，Erg 的語法可能在版本 0.* 期間發生破壞性更改，并且文檔可能沒有相應更新。請事先了解這一點
> 如果您在本文檔中發現任何錯誤，請報告至 [此處的表單](https://forms.gle/HtLYRfYzWCAaeTGb6) 或 [GitHub repo](https://github.com/mtshiba/TheErgBook/issues/new )。我們將不勝感激您的建議

本文檔描述 Erg 的基本語法
如果您已經有使用 Python 等語言的經驗，請參閱 [快速瀏覽](quick_tour.md) 了解概覽
還有一個單獨的 [標準 API](../API/index.md) 和 [Erg 貢獻者的內部文檔](../dev_guide/index.md)。如果您需要語法或 Erg 本身的詳細說明, 請參閱那些文檔

## 你好，世界&excl;

首先，讓我們做"Hello World"

```python
print!("Hello, World!")
```

這與 Python 和同一家族中的其他語言幾乎相同。最顯著的特征是`!`，后面會解釋它的含義
在 Erg 中，括號 `()` 可以省略，除非在解釋上有一些混淆
括號的省略與 Ruby 類似，但不能省略可以以多種方式解釋的括號

```python
print! "Hello, World!" # OK
print! "Hello,", "World!" # OK
print!() # OK
print! # OK, 但這并不意味著調用，只是將 `print!` 作為可調用對象

print! f x # OK, 解釋為 `print!(f(x))`
print!(f(x, y)) # OK
print! f(x, y) # OK
print! f(x, g y) # OK
print! f x, y # NG, 可以理解為 `print!(f(x), y)` 或 `print!(f(x, y))` print!
print!(f x, y) # NG, 可以表示"print！(f(x)，y)"或"print！(f(x，y))"
print! f(x, g y, z) # NG, 可以表示"print！(x，g(y)，z)"或"print！(x，g(y，z))"
```

## 腳本

Erg 代碼稱為腳本。腳本可以以文件格式 (.er) 保存和執行

## REPL/文件執行

要啟動 REPL，只需鍵入: 

```sh
> erg
```

`>` mark is a prompt, just type `erg`.
Then the REPL should start.

```sh
> erg
Starting the REPL server...
Connecting to the REPL server...
Erg interpreter 0.2.4 (tags/?:, 2022/08/17  0:55:12.95) on x86_64/windows
>>>
```

Or you can compile from a file.

```sh
> 'print! "hello, world!"' >> hello.er

> erg hello.er
hello, world!
```

## 注釋

`#` 之后的代碼作為注釋被忽略。使用它來解釋代碼的意圖或暫時禁用代碼

```python
# Comment
# `#` and after are ignored until a new line is inserted
# [
Multi-line comment
Treated as a comment all the way up to the corresponding `]# `
]# 
```

## 表達式，分隔符

腳本是一系列表達式。表達式是可以計算或評估的東西，在 Erg 中幾乎所有東西都是表達式
每個表達式由分隔符分隔 - 新行或分號 `;`-
Erg 腳本基本上是從左到右、從上到下進行評估的

```python
n = 1 # 賦值表達式
f(1, 2) # 函數調用表達式
1 + 1 # 運算符調用表達式
f(1, 2); 1 + 1
```

如下所示，有一種稱為 Instant block 的語法，它將塊中評估的最后一個表達式作為變量的值
這與沒有參數的函數不同，它不添加 `()`。請注意，即時塊僅在運行中評估一次

```python
i =
    x = 1
    x + 1
assert i == 2
```

這不能用分號 (`;`) 完成

```python
i = (x = 1; x + 1) # 語法錯誤: 不能在括號中使用 `;`
```

## 縮進

Erg 和 Python 一樣，使用縮進來表示塊。有五個運算符(特殊形式)觸發塊的開始: `=`、`->`、`=>`、`do` 和 `do!`(此外，`:` 和 `|` ，雖然不是運算符，但也會產生縮進)。每個的含義將在后面描述

```python
f x, y =
    x + y

for! 0..9, i =>
    print!

for! 0..9, i =>
    print! i; print! i

ans = match x:
    0 -> "zero"
    _: 0..9 -> "1 dight"
    _: 10..99 -> "2 dights"
    _ -> "unknown"
```

如果一行太長，可以使用 `\` 將其斷開

```python
# 這不是表示 `x + y + z` 而是表示 `x; +y; +z`
X
+ y
+ z

# 這意味著`x + y + z`
x \
+ y \
+ z
```

<p align='center'>
    上一頁 | <a href='./01_literal.md'>下一頁</a>
</p>

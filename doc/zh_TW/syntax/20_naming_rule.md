# 命名規則

如果要將變量用作常量表達式，則必須以大寫字母開頭。兩個以上的字符可以是小寫的。


```erg
i: Option Type = Int
match i:
    t: Type -> log "type"
    None -> log "None"
```

具有副作用的對象始終以結尾。過程和過程方法，以及變量類型。但是，<gtr=“7”/>類型本身不是可變類型。


```erg
# Callable == Func or Proc
c: Callable = print!
match c:
    p! -> log "proc" #`:Proc` 可以省略，因為它是不言自明的
    f -> log "func"
```

如果你想要將屬性公開到外部，請首先使用進行定義。如果未在開始時添加<gtr=“9”/>，則不公開。不能在同一範圍內共存，以避免混淆。


```erg
o = {x = 1; .x = 2} # SyntaxError: private and public variables with the same name cannot coexist
```

## 文字標識符（Literal Identifiers）

可以通過將字符串括在單引號（‘’）中來避免上述規則。也就是說，過程對像也可以賦值，而不使用。但是，即使值是常量表達式，也不會將其視為常量。這種用單引號括起來的字符串標識符稱為文字標識符。它用於調用其他語言的 API(FFI)，如 Python。


```erg
bar! = pyimport("foo").'bar'
```

如果標識符對 Erg 也有效，則不需要用‘’括起來。

此外，由於文字標識符可以包含符號和空格，因此通常不能用作標識符的字符串可以用作標識符。


```erg
'∂/∂t' y
'test 1: pass x to y'()
```

<p align='center'>
    <a href='./19_visibility.md'>Previous</a> | <a href='./21_lambda.md'>Next</a>
</p>
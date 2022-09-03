# 命名规则

如果要将变量用作常量表达式，则必须以大写字母开头。两个以上的字符可以是小写的。


```erg
i: Option Type = Int
match i:
    t: Type -> log "type"
    None -> log "None"
```

具有副作用的对象始终以结尾。过程和过程方法，以及变量类型。但是，<gtr=“7”/>类型本身不是可变类型。


```erg
# Callable == Func or Proc
c: Callable = print!
match c:
    p! -> log "proc" #`:Proc` 可以省略，因为它是不言自明的
    f -> log "func"
```

如果你想要将属性公开到外部，请首先使用进行定义。如果未在开始时添加<gtr=“9”/>，则不公开。不能在同一范围内共存，以避免混淆。


```erg
o = {x = 1; .x = 2} # SyntaxError: private and public variables with the same name cannot coexist
```

## 文字标识符（Literal Identifiers）

可以通过将字符串括在单引号（‘’）中来避免上述规则。也就是说，过程对象也可以赋值，而不使用。但是，即使值是常量表达式，也不会将其视为常量。这种用单引号括起来的字符串标识符称为文字标识符。它用于调用其他语言的 API(FFI)，如 Python。


```erg
bar! = pyimport("foo").'bar'
```

如果标识符对 Erg 也有效，则不需要用‘’括起来。

此外，由于文字标识符可以包含符号和空格，因此通常不能用作标识符的字符串可以用作标识符。


```erg
'∂/∂t' y
'test 1: pass x to y'()
```

<p align='center'>
    <a href='./19_visibility.md'>Previous</a> | <a href='./21_lambda.md'>Next</a>
</p>

# 命名约定

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/20_naming_rule.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/20_naming_rule.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

如果要将变量用作常量表达式，请确保它以大写字母开头。两个或多个字母可能是小写的

```python
i: Option Type = Int
match i:
    t: Type -> log "type"
    None -> log "None"
```

具有副作用的对象总是以 `!` 结尾。程序和程序方法，以及可变类型
然而，`Proc` 类型本身是不可变的

```python
# Callable == Func or Proc
c: Callable = print!
match c:
    p! -> log "proc" # `: Proc` 可以省略，因为它是不言自明的
    f -> log "func"
```

如果您想向外界公开一个属性，请在开头使用 `.` 定义它。如果你不把`.`放在开头，它将是私有的。为避免混淆，它们不能在同一范围内共存

```python
o = {x = 1; .x = 2} # 语法错误: 同名的私有变量和公共变量不能共存
```

## 文字标识符

可以通过将字符串括在单引号 ('') 中来规避上述规则。也就是说，程序对象也可以在没有 `!` 的情况下分配。但是，在这种情况下，即使该值是常量表达式，也不会被视为常量
像这样用单引号括起来的字符串称为文字标识符
这在调用Python等其他语言的API(FFI)时使用

```python
bar! = pyimport("foo").'bar'
```

在 Erg 中也有效的标识符不需要用 '' 括起来

此外，文字标识符可以包含符号和空格，因此通常不能用作标识符的字符串可以用作标识符

```python
'∂/∂t' y
'test 1: pass x to y'()
```

<p align='center'>
    <a href='./19_visibility.md'>上一页</a> | <a href='./21_lambda.md'>下一页</a>
</p>
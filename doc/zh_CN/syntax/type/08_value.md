# 值类型

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/08_value.md%26commit_hash%3Db713e6f5cf9570255ccf44d14166cb2a9984f55a)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/08_value.md&commit_hash=b713e6f5cf9570255ccf44d14166cb2a9984f55a)

值类型是可以在编译时评估的 Erg 内置类型，具体来说:

```python
Value = (
    Int
    or Nat
    or Ratio
    or Float
    or Complex
    or Bool
    or Str
    or NoneType
    or List Const
    or Tuple Const
    or Set Const
    or ConstFunc(Const, _)
    or ConstProc(Const, _)
    or ConstMethod(Const, _)
)
```

应用于它们的值类型对象、常量和编译时子例程称为 __constant 表达式__

```python
1, 1.0, 1+2im, True, None, "aaa", [1, 2, 3], Fib(12)
```

小心子程序。子例程可能是也可能不是值类型
由于子程序的实质只是一个指针，因此可以将其视为一个值[<sup id="f1">1</sup>](#1)，但是在编译不是子程序的东西时不能使用 在恒定的上下文中。不是值类型，因为它没有多大意义

将来可能会添加归类为值类型的类型

---

<span id="1" style="font-size:x-small"><sup>1</sup> Erg 中的术语"值类型"与其他语言中的定义不同。纯 Erg 语义中没有内存的概念，并且因为它被放置在堆栈上而说它是值类型，或者因为它实际上是一个指针而说它不是值类型是不正确的。值类型仅表示它是"值"类型或其子类型。[↩](#f1)</span>

<p align='center'>
    <a href='./07_patch.md'>上一页</a> | <a href='./09_attributive.md'>下一页</a>
</p>

# Num

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/types/traits/Num.md%26commit_hash%3D14710744ed4c3aa29a43953366c67162bc157f7d)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/types/traits/Num.md&commit_hash=14710744ed4c3aa29a43953366c67162bc157f7d)


`A<：B`表示类型A是类型B的子类型声明。此外, 类型A此时称为子类型, 类型B称为广义类型(超类型)。此外, 如果`A<：B`, 则类型为A的所有表达式都具有类型B的属性。这称为包含(subsumption)

Erg内置数字类型的类型关系如下:

- 布尔类型(Bool) <: 自然数类型(Nat) <: 整数类型(Int) <: 有理数类型(Ratio) <: 复合数类型(Complex)

指数文字是有理文字的另一种表示形式, 并且属于同一类型

计算时，根据情况进行向上转换(向下转换)

> __注意__: 在当前的实现中，浮点类并不作为一个单独的类存在，而是以与有理字面量相同的方式实现。将来, 这个浮点类将再次作为一个单独的类实现, 以加快计算速度
> 此外，复杂对象目前是使用浮点对象实现的, 将来也会用有理字面量重写

```python
>>> 1 + 1.0 # Nat(Int)+Ratio 向上转换为 Ratio+Ratio
2.0 # Ratio
>>> 10.0 // 2 # Ratio//Nat(Int) 也向上转换为 Ratio//Ratio. Ratio//Ratio 的结果是 Int
5 # Int(Nat)
>>> True == 1.0 # Bool==Ratio 向上转换为 Ratio==Ratio
True
```

如果未指定类型, 则推断它们以便它们向上转换为相同类型
一般来说, 向下转换是不安全的，转换方法也很重要

以后不能重新定义类之间的转换。只有在定义类时通过继承指定超类时, 它才有资格进行强制转换
此外，特征不能被部分类型化, 除非它们在类定义时基本上“实现”。但是，这只能通过 [patch](../../../syntax/type/07_patch.md) 来完成

如果协变复合文字(例如数组文字)处于包含关系中, 则可以进行强制转换。
但是请注意，具有非退化的类型不能在 Erg 中强制转换，即使它们处于包含关系中(有关详细信息，请参阅 [degenerate](../../../syntax/type/advanced/variance.md))

## 定义

```python
Num R = Add(R) and Sub(R) and Mul(R) and Eq
Num = Num Self
```

## 父类(超类)

`Add`, `Sub`, `Mul` 和 `Eq`

## 方法

*`abs`

# 对象系统

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/25_object_system.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/25_object_system.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

可以分配给变量的所有数据。`Object` 类的属性如下

* `.__repr__`: 返回对象的(非丰富)字符串表示
* `.__sizeof__`: 返回对象的大小(包括堆分配)
* `.__dir__`: 返回对象属性列表
* `.__hash__`: 返回对象的哈希值
* `.__getattribute__`: 获取并返回对象的属性
* `.clone`: 创建并返回一个对象的克隆(在内存中有一个独立的实体)
* `.copy`: 返回对象的副本(指向内存中的同一事物)

## 记录

由记录文字(`{attr = value; ...}`)生成的对象
这个对象有基本的方法，比如`.clone`和`.__sizeof__`

```python
obj = {.x = 1}
assert obj.x == 1

obj2 = {...x; .y = 2}
assert obj2.x == 1 and obj2.y == 2
```

## 属性

与对象关联的对象。特别是，将 self (`self`) 作为其隐式第一个参数的子例程属性称为方法

```python
# 请注意，private_attr 中没有`.`
record = {.public_attr = j; private_attr = 2; .method = self -> self.i + 1}
record. public_attr == 2
record.private_attr # AttributeError: private_attr 是私有的
assert record.method() == 3
```

## 元素

属于特定类型的对象(例如，"1"是"Int"类型的元素)。所有对象至少是`{=}`类型的元素
类的元素有时称为实例

## 子程序

表示作为函数或过程(包括方法)实例的对象。代表子程序的类是"子程序"
实现 `.__call__` 的对象通常称为 `Callable`

## 可调用

一个实现`.__call__`的对象。它也是 `Subroutine` 的父类

## 类型

定义需求属性并使对象通用化的对象
主要有两种类型: 多态类型和单态类型。典型的单态类型有`Int`、`Str`等，多态类型有`Option Int`、`[Int; 3]`等
此外，定义改变对象状态的方法的类型称为 Mutable 类型，需要在变量属性中添加 `!`(例如动态数组: `[T; !_]`)

## 班级

具有 `.__new__`、`.__init__` 方法等的类型。实现基于类的面向对象

## 功能

对外部变量(不包括静态变量)有读权限但对外部变量没有读/写权限的子程序。换句话说，它没有外部副作用
Erg 函数的定义与 Python 的不同，因为它们不允许副作用

## 程序

它对外部变量具有读取和"自我"权限，对静态变量具有读/写权限，并允许使用所有子例程。它可能有外部副作用

## 方法

隐式将"self"作为第一个参数的子例程。它与简单的函数/过程是不同的类型

## 实体

不是子例程和类型的对象
单态实体(`1`、`"a"` 等)也称为值对象，多态实体(`[1, 2, 3], {"a": 1}`)也称为容器对象

<p align='center'>
    <a href='./24_module.md'>上一页</a> | <a href='./26_pattern_matching.md'>下一页</a>
</p>
# 记录(Record)

记录是一个集合，它结合了通过键访问的 Dict 和在编译时检查其访问的元组的属性。
如果您了解 JavaScript，请将其视为一种（更增强的）对象字面量表示法。

```python
john = {.name = "John"; .age = 21}

assert john.name == "John"
assert john.age == 21
assert john in {.name = Str; .age = Nat}
john["name"] # 错误：john 不可订阅
```

`.name` 和 `.age` 部分称为属性，而 `"John"` 和 `21` 部分称为属性值。

与 JavaScript 对象字面量的区别在于它们不能作为字符串访问。 也就是说，属性不仅仅是字符串。
这是因为对值的访问是在编译时确定的，而且字典和记录是不同的东西。 换句话说，`{"name": "John"}` 是一个字典，`{name = "John"}` 是一个记录。
那么我们应该如何使用字典和记录呢？
一般来说，我们建议使用记录。 记录具有在编译时检查元素是否存在以及能够指定 __visibility_ 的优点。
指定可见性等同于在 Java 和其他语言中指定公共/私有。 有关详细信息，请参阅 [可见性](./15_visibility.md) 了解详细信息。

```python
a = {x = 1; .y = x + 1}
a.x # 属性错误：x 是私有的
# 提示：声明为 `.x`。
assert a.y == 2
```

对于熟悉 JavaScript 的人来说，上面的示例可能看起来很奇怪，但简单地声明 `x` 会使其无法从外部访问

您还可以显式指定属性的类型

```python
anonymous = {
    .name: Option! Str = !
    .age = 20
}
anonymous.name.set! "John"
```

一个记录也可以有方法。

```python
o = {
    .i = !0
    .inc! ref! self = self.i.inc!()
}

assert o.i == 0
o.inc!()
assert o.i == 1
```

关于记录有一个值得注意的语法。 当记录的所有属性值都是类（不是结构类型）时，记录本身表现为一个类型，其自身的属性作为必需属性。
这种类型称为记录类型。 有关详细信息，请参阅 [记录] 部分。

```python
# 记录
john = {.name = "John"}
# 记录 type
john: {.name = Str}
Named = {.name = Str}
john: Named

greet! n: Named =
    print! "Hello, I am {n.name}"
john # “你好，我是约翰 print！

Named.name # Str
```

## 解构记录

记录可以按如下方式解构。

```python
record = {x = 1; y = 2}
{x = a; y = b} = record
assert a == 1
assert b == 2

point = {x = 2; y = 3; z = 4}
match point:
    {x = 0; y = 0; z = 0} -> "origin"
    {x = _; y = 0; z = 0} -> "on the x axis"
    {x = 0; ...} -> "x = 0"
    {x = x; y = y; z = z} -> "({x}, {y}, {z})"
```

当存在与属性同名的变量时，`x = ...`也可以缩写为`x`，例如`x = x`或`x = .x`到`x`，和` .x = .x` 或 `.x = x` 到 `.x`。
但是，当只有一个属性时，必须在其后加上`;`以与集合区分开来。

```python
x = 1
y = 2
xy = {x; y}
a = 1
b = 2
ab = {.a; .b}
assert ab.a == 1
assert ab.b == 2

record = {x;}
tuple = {x}
assert tuple.1 == 1
```

此语法可用于解构记录并将其分配给变量

```python
# 一样 `{x = x; y = y} = xy`
{x; y} = xy
assert x == 1
assert y == 2
# 一样 `{.a = a; .b = b} = ab`
{a; b} = ab
assert a == 1
assert b == 2
```

## 空记录

空记录由`{=}`表示。 空记录也是它自己的类，如 Unit

```python
empty_record = {=}
empty_record: {=}
# Object: Type = {=}
empty_record: Object
empty_record: Structural {=}
{x = 3; y = 5}: Structural {=}
```

空记录不同于空 Dict `{:}` 或空集 `{}`。 特别要注意的是，它与 `{}` 的含义相反（在 Python 中，`{}` 是一个空字典，而在 Erg 中它是 Erg 中的 `!{:}`）。
作为枚举类型，`{}` 是一个空类型，其元素中不包含任何内容。 `Never` 类型是这种类型的一个分类。
相反，记录类 `{=}` 没有必需的实例属性，因此所有对象都是它的元素。 `Object` 是 this 的别名。
一个`Object`（`Object`的一个补丁）是`的一个元素。 __sizeof__` 和其他非常基本的提供方法。

```python
AnyPatch = Patch Structural {=}
    . __sizeof__ self = ...
    .clone self = ...
    ...
Never = Class {}
```

请注意，没有其他类型或类在结构上与 `{}`、`Never` 类型等效，如果用户在右侧使用 `{}`、`Class {}` 定义类型，则会出错。
这意味着，例如，`1..10 或 -10。 -1`，但 `1..10 和 -10... -1`。 例如，当它应该是 1..10 或 -10...-1 时是 `-1`。
此外，如果您定义的类型（例如 `Int 和 Str`）会导致组合 `Object`，则会警告您只需将其设置为 `Object`。

## 即时封锁

Erg 有另一种语法 Instant 块，它只返回最后评估的值。 不能保留属性。

```python
x =
    x = 1
    y = x + 1
    y ** 3
assert x == 8

y =
    .x = 1 # 语法错误：无法在实体块中定义属性
```

## 数据类

如果您尝试自己实现方法，则必须直接在实例中定义裸记录（由记录文字生成的记录）。
这是低效的，并且随着属性数量的增加，错误消息等变得难以查看和使用。

```python
john = {
    name = "John Smith"
    age = !20
    .greet! ref self = print! "Hello, my name is {self::name} and I am {self::age} years old."
    .inc_age! ref! self = self::age.update! x -> x + 1
}
john + 1
# 类型错误：{name = Str; 没有实现 + 年龄=诠释； 。迎接！ =参考（自我）。 () => 无； inc_age！ =参考！ () => 无}, 整数
```

因此，在这种情况下，您可以继承一个记录类。 这样的类称为数据类。
这在 [class](./type/04_class.md) 中有描述。

```python
Person = Inherit {name = Str; age = Nat}
Person.
    greet! ref self = print! "Hello, my name is {self::name} and I am {self::age} years old."
    inc_age! ref! self = self::age.update! x -> x + 1

john = Person.new {name = "John Smith"; age = 20}
john + 1
# 类型错误：Person、Int 没有实现 +
```

<p align='center'>
    <a href='./12_tuple.md'>上一页</a> | <a href='./14_set.md'>下一页</a>
</p>

# 补丁

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/07_patch.md%26commit_hash%3Dbade70ef91c040f40cb181399ad7056527d9a1c5)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/07_patch.md&commit_hash=bade70ef91c040f40cb181399ad7056527d9a1c5)

Erg 不允许修改现有类型和类
这意味着，不可能在类中定义额外的方法，也不能执行特化(一种语言特性，单态化多态声明的类型并定义专用方法，如在 C++ 中)
但是，在许多情况下，您可能希望向现有类型或类添加功能，并且有一个称为"修补"的功能允许您执行此操作

```python
StrReverse = Patch Str
StrReverse.
    reverse self = self.iter().rev().collect(Str)

assert "abc".reverse() == "cba"
```

补丁的名称应该是要添加的主要功能的简单描述
这样，被修补类型的对象(`Str`)可以使用修补程序的方法(`StrReverse`)
实际上，内置方法`.reverse`并不是`Str`的方法，而是`StrRReverse`中添加的方法

但是，补丁方法的优先级低于名义类型(类/trait)的方法，并且不能覆盖现有类型

```python
StrangeInt = Patch Int
StrangeInt.
    `_+_` = Int.`_-_` # 赋值错误: . `_+_` 已在 Int 中定义
```

如果要覆盖，则必须从类继承
但是，基本上建议不要覆盖并定义具有不同名称的方法
由于一些安全限制，覆盖不是很容易做到

```python
StrangeInt = Inherit Int
StrangeInt.
# 覆盖方法必须被赋予覆盖装饰器
    # 另外，你需要覆盖所有依赖于 Int.`_+_` 的 Int 方法
    @Override
    `_+_` = Super.`_-_` # OverrideError: Int.`_+_` 被 ... 引用，所以这些方法也必须被覆盖
```

## 选择修补程序

可以为单一类型定义修复程序，并且可以组合在一起

```python
# foo.er

StrReverse = Patch(Str)
StrReverse.
    reverse self = ...
StrMultiReplace = Patch(Str)
StrMultiReverse.
    multi_replace self, pattern_and_targets: [(Pattern, Str)] = ...
StrToCamelCase = Patch(Str)
StrToCamelCase.
    to_camel_case self = ...
StrToKebabCase = Patch(Str)
StrToKebabCase.
    to_kebab_case self = ...

StrBoosterPack = StrReverse and StrMultiReplace and StrToCamelCase and StrToKebabCase
StrBoosterPack = StrReverse and StrMultiReplace and StrToCamelCase and StrToKebabCase
```

```python
{StrBoosterPack; ...} = import "foo"

assert "abc".reverse() == "cba"
assert "abc".multi_replace([("a", "A"), ("b", "B")]) == "ABc"
assert "to camel case".to_camel_case() == "toCamelCase"
assert "to kebab case".to_kebab_case() == "to-kebab-case"
```

如果定义了多个修复程序，其中一些可能会导致重复实施

```python
# foo.er

StrReverse = Patch(Str)
StrReverse.
    reverse self = ...
# 更高效的实现
StrReverseMk2 = Patch(Str)
StrReverseMk2.
    reverse self = ...

"hello".reverse() # 补丁选择错误: `.reverse` 的多个选择: StrReverse, StrReverseMk2
```

在这种情况下，您可以使用 __related function__ 形式而不是方法形式使其唯一

```python
assert StrReverseMk2.reverse("hello") == "olleh"
```

You can also make it unique by selectively importing.

```python
{StrReverseMk2; ...} = import "foo"

assert "hello".reverse() == "olleh"
```

## 胶水补丁

维修程序也可以将类型相互关联。`StrReverse` 补丁涉及 `Str` 和 `Reverse`
这样的补丁称为 __glue patch__
因为 `Str` 是内置类型，所以用户需要使用胶水补丁来改造Trait

```python
Reverse = Trait {
    .reverse = Self.() -> Self
}

StrReverse = Patch Str, Impl := Reverse
StrReverse.
    reverse self =
        self.iter().rev().collect(Str)
```

每个类型/Trait对只能定义一个胶水补丁
这是因为如果多个胶水修复程序同时"可见"，就不可能唯一确定选择哪个实现
但是，当移动到另一个范围(模块)时，您可以交换维修程序

```python
NumericStr = Inherit Str
NumericStr.
    ...

NumStrRev = Patch NumericStr, Impl := Reverse
NumStrRev.
    ...
# 重复修补程序错误: 数值Str已与"反向"关联`
# 提示: 'Str'(NumericStr'的父类)通过'StrReverse'与'Reverse'关联
```

## 附录: 与 Rust Trait的关系

Erg 修复程序相当于 Rust 的(改造的)`impl` 块

```rust
// Rust
trait Reverse {
    fn reverse(self) -> Self;
}

impl Reverse for String {
    fn reverse(self) -> Self {
        self.chars().rev().collect()
    }
}
```

可以说，Rust 的Trait是 Erg 的Trait和修复程序的Trait。这使得 Rust 的Trait听起来更方便，但事实并非如此

```python
# Erg
Reverse = Trait {
    .reverse = Self.() -> Self
}

StrReverse = Patch(Str, Impl := Reverse)
StrReverse.
    reverse self =
        self.iter().rev().collect(Str)
```

因为 impl 块在 Erg 中被对象化为补丁，所以在从其他模块导入时可以选择性地包含。作为副作用，它还允许将外部Trait实现到外部结构
此外，结构类型不再需要诸如 `dyn trait` 和 `impl trait` 之类的语法

```python
# Erg
reversible: [Reverse; 2] = [[1, 2, 3], "hello"]

iter|T|(i: Iterable T): Iterator T = i.iter()
```

```rust
// Rust
let reversible: [Box<dyn Reverse>; 2] = [Box::new([1, 2, 3]), Box::new("hello")];

fn iter<I>(i: I) -> impl Iterator<Item = I::Item> where I: IntoIterator {
    i.into_iter()
}
```

## 通用补丁

补丁不仅可以为一种特定类型定义，还可以为"一般功能类型"等定义
在这种情况下，要给出自由度的项作为参数给出(在下面的情况下，`T: Type`)。以这种方式定义的补丁称为全对称补丁
如您所见，全对称补丁正是一个返回补丁的函数，但它本身也可以被视为补丁

```python
FnType T: Type = Patch(T -> T)
FnType(T).
    type = T

assert (Int -> Int).type == Int
```

## 结构补丁

此外，可以为满足特定结构的任何类型定义修复程序
但是，这比名义上的维修程序和类方法具有较低的优先级

在定义结构修复程序时应使用仔细的设计，因为某些属性会因扩展而丢失，例如以下内容

```python
# 这不应该是 `Structural`
Norm = Structural Patch {x = Int; y = Int}
Norm.
    norm self = self::x**2 + self::y**2

Point2D = Class {x = Int; y = Int}
assert Point2D.new({x = 1; y = 2}).norm() == 5

Point3D = Class {x = Int; y = Int; z = Int}
assert Point3D.new({x = 1; y = 2; z = 3}).norm() == 14 # AssertionError:
```

<p align='center'>
    <a href='./06_nst_vs_sst.md'>上一页</a> | <a href='./08_value.md'>下一页</a>
</p>

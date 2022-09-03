# Patch

Erg 不允许修改现有类型类。不能在类中定义额外的方法，而是专门化（specialization，将声明为多相的类型单相化并定义专用方法的功能。C++ 等也不能使用。但是，在许多情况下，你希望将功能添加到现有类型类中，而修补程序就是实现这一目标的方法。


```erg
StrReverse = Patch Str
StrReverse.
    reverse self = self.iter().rev().collect(Str)

assert "abc".reverse() == "cba"
```

修补程序的名称最好是要添加的主要功能的直接描述。这样，要修补的类型（）的对象就可以使用修补方法（<gtr=“18”/>）。实际上，<gtr=“19”/>不是<gtr=“20”/>方法，而是添加到<gtr=“21”/>中的方法。

但是，修补程序方法的优先级低于记名（类）方法，因此不能覆盖（覆盖）现有类的方法。


```erg
StrangeInt = Patch Int
StrangeInt.
    `_+_` = Int.`_-_` # AssignError: .`_+_` is already defined in Int
```

如果要覆盖，必须继承类。但是，建议你定义一个具有不同名称的方法，而不是覆盖它。因为覆盖有一些安全限制，不是那么容易做到的。


```erg
StrangeInt = Inherit Int
StrangeInt.
    # オーバーライドするメソッドにはOverrideデコレータを付与する必要がある
    # さらに、Int.`_+_`に依存するIntのメソッドすべてをオーバーライドする必要がある
    @Override
    `_+_` = Super.`_-_` # OverrideError: Int.`_+_` is referenced by ..., so these method must also be overridden
```

## 选择修补程序

可以为一种类型定义多个曲面片，也可以将它们组合在一起。


```erg
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
```


```erg
{StrBoosterPack; ...} = import "foo"

assert "abc".reverse() == "cba"
assert "abc".multi_replace([("a", "A"), ("b", "B")]) == "ABc"
assert "to camel case".to_camel_case() == "toCamelCase"
assert "to kebab case".to_kebab_case() == "to-kebab-case"
```

如果可以定义多个修补程序，某些修补程序可能会导致重复的实现。


```erg
# foo.er

StrReverse = Patch(Str)
StrReverse.
    reverse self = ...
# more efficient implementation
StrReverseMk2 = Patch(Str)
StrReverseMk2.
    reverse self = ...

"hello".reverse() # PatchSelectionError: multiple choices of `.reverse`: StrReverse, StrReverseMk2
```

在这种情况下，可以使用相关函数格式而不是方法格式来实现唯一性。


```erg
assert StrReverseMk2.reverse("hello") == "olleh"
```

也可以通过选择性导入来实现唯一性。


```erg
{StrReverseMk2; ...} = import "foo"

assert StrReverseMk2.reverse("hello") == "olleh"
```

## 粘合面片（Glue Patch）

修补程序还可以关联类型。将<gtr=“23”/>与<gtr=“24”/>关联起来。这些面片称为“粘合面片”（Glue Patch）。由于<gtr=“25”/>是一个内置类型，因此用户需要一个粘合贴片来改装托盘。


```erg
Reverse = Trait {
    .reverse = Self.() -> Self
}

StrReverse = Patch Str, Impl := Reverse
StrReverse.
    reverse self =
        self.iter().rev().collect(Str)
```

只能为一对类型和托盘定义一个粘合曲面片。这是因为，如果多个粘合贴片同时“可见”，则无法唯一确定选择哪个实现。但是，你可以在切换到其他范围（模块）时替换修补程序。


```erg
NumericStr = Inherit Str
NumericStr.
    ...

NumStrRev = Patch NumericStr, Impl := Reverse
NumStrRev.
    ...
# DuplicatePatchError: NumericStr is already associated with `Reverse`
# hint: `Str` (superclass of `NumericStr`) is associated with `Reverse` by `StrReverse`
```

## Appendix：Rust 与特雷特的关系

Erg 修补程序相当于 Rust 的 impl 块（后置）。


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

可以说，Rust Traitt 是 Erg Traitt 和补丁的功能的结合。这样说来，Rust 的特雷特听起来更方便，其实也不尽然。


```erg
# Erg
Reverse = Trait {
    .reverse = Self.() -> Self
}

StrReverse = Patch(Str, Impl := Reverse)
StrReverse.
    reverse self =
        self.iter().rev().collect(Str)
```

Erg 将 impl 块对象化为修补程序，以便在从其他模块导入时进行选择性导入。此外，还允许在外部结构中实现外部托盘。此外，由于结构类型的不同，也不需要 dyn trait 和 impl trait 语法。


```erg
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

## 全称补丁

你可以为特定类型定义修补程序，也可以为“常规函数类型”等定义修补程序。在这种情况下，将想要给出自由度的项作为参数（在下面的情况下为）。以这种方式定义的曲面片称为全称曲面片。正如你所看到的，全称修补程序是一个返回修补程序的函数，但它本身也可以被视为修补程序。


```erg
FnType T: Type = Patch(T -> T)
FnType(T).
    type = T

assert (Int -> Int).type == Int
```

## 结构补丁

此外，还可以为满足某一结构的所有类型定义修补程序。但是，它的优先级低于记名修补程序和类方法。

在定义结构修补程序时，请仔细设计，因为扩展可能会导致不成立，如下所示。


```erg
# これはStructuralにするべきではない
Norm = Structural Patch {x = Int; y = Int}
Norm.
    norm self = self::x**2 + self::y**2

Point2D = Class {x = Int; y = Int}
assert Point2D.new({x = 1; y = 2}).norm() == 5

Point3D = Class {x = Int; y = Int; z = Int}
assert Point3D.new({x = 1; y = 2; z = 3}).norm() == 14 # AssertionError:
```

<p align='center'>
    <a href='./06_nst_vs_sst.md'>Previous</a> | <a href='./08_value.md'>Next</a>
</p>

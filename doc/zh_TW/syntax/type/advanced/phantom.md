# 幻影(phantom)类

幻像类型是标记特征，其存在仅用于向编译器提供注释。
作为幻像类型的一种用法，让我们看一下列表的结构。

```python
Nil = Class()
List T, 0 = Inherit Nil
List T, N: Nat = Class {head = T; rest = List(T, N-1)}
```

此代码导致错误。

```python
3 | List T, 0 = Inherit Nil
                        ^^^
类型构造错误：由于Nil没有参数T，所以无法用Nil构造List(T, 0)
提示：使用 'Phantom' 特质消耗 T
```

此错误是在使用 `List(_, 0).new Nil.new()` 时无法推断 `T` 的抱怨。
在这种情况下，无论 `T` 类型是什么，它都必须在右侧使用。 大小为零的类型(例如长度为零的元组)很方便，因为它没有运行时开销。
```python
Nil T = Class((T; 0))
List T, 0 = Inherit Nil T
List T, N: Nat = Class {head = T; rest = List(T, N-1)}
```

此代码通过编译。 但是理解意图有点棘手，除非类型参数是类型，否则不能使用它。

在这种情况下，幻影类型正是您所需要的。 幻像类型是大小为 0 的广义类型。

```python
Nil T = Class(Impl := Phantom T)
List T, 0 = Inherit Nil T
List T, N: Nat = Class {head = T; rest = List(T, N-1)}

nil = Nil(Int).new()
assert nil.__size__ == 0
```

`Phantom` 拥有`T` 类型。 但实际上 `Phantom T` 类型的大小是 0 并且不包含 `T` 类型的对象。

此外，`Phantom` 可以使用除其类型之外的任意类型参数。 在下面的示例中，`Phantom` 包含一个名为 `State` 的类型参数，它是 `Str` 的子类型对象。
同样，`State` 是一个假的类型变量，不会出现在对象的实体中。

```python
VM! State: {"stopped", "running"}! = Class(... State)
VM!("stopped").
    start ref! self("stopped" ~> "running") =
        self.do_something!()
        self::set_phantom!("running"))
```

`state` 是通过 `update_phantom!` 或 `set_phantom!` 方法更新的。
这是标准补丁为`Phantom!`(`Phantom`的变量版本)提供的方法，其用法与变量`update!`和`set!`相同。
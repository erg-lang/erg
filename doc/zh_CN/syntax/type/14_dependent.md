# 依赖类型

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/14_dependent.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/14_dependent.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

依赖类型是一个特性，可以说是 Erg 的最大特性。
依赖类型是将值作为参数的类型。 普通的多态类型只能将类型作为参数，但依赖类型放宽了这个限制。

依赖类型等价于`[T; N]`(`数组(T，N)`)。
这种类型不仅取决于内容类型"T"，还取决于内容数量"N"。 `N` 包含一个`Nat` 类型的对象。

```python
a1 = [1, 2, 3]
assert a1 in [Nat; 3]
a2 = [4, 5, 6, 7]
assert a1 in [Nat; 4]
assert a1 + a2 in [Nat; 7]
```

如果函数参数中传递的类型对象与返回类型有关，则写：

```python
narray: |N: Nat| {N} -> [{N}; N]
narray(N: Nat): [N; N] = [N; N]
assert array(3) == [3, 3, 3]
```

定义依赖类型时，所有类型参数都必须是常量。

依赖类型本身存在于现有语言中，但 Erg 具有在依赖类型上定义过程方法的特性

```python
x=1
f x =
    print! f::x, module::x

# Phantom 类型有一个名为 Phantom 的属性，其值与类型参数相同
T X: Int = Class Impl := Phantom X
T(X).
    x self = self::Phantom

T(1).x() # 1
```

可变依赖类型的类型参数可以通过方法应用程序进行转换。
转换规范是用 `~>` 完成的

```python
# 注意 `Id` 是不可变类型，不能转换
VM!(State: {"stopped", "running"}! := _, Id: Nat := _) = Class(..., Impl := Phantom! State)
VM!().
    # 不改变的变量可以通过传递`_`省略。
    start! ref! self("stopped" ~> "running") =
        self.initialize_something!()
        self::set_phantom!("running")

# 你也可以按类型参数切出(仅在定义它的模块中)
VM!.new() = VM!(!"stopped", 1).new()
VM!("running" ~> "running").stop!ref!self =
    self.close_something!()
    self::set_phantom!("stopped")

vm = VM!.new()
vm.start!()
vm.stop!()
vm.stop!() # 类型错误：VM!(!"stopped", 1) 没有 .stop!()
# 提示：VM!(!"running", 1) 有 .stop!()
```

您还可以嵌入或继承现有类型以创建依赖类型。

```python
MyArray(T, N) = Inherit[T; N]

# self 的类型：Self(T, N) 与 .array 一起变化
MyStruct!(T, N: Nat!) = Class {.array: [T; !N]}
```
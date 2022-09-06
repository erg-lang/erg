# 枚举类型

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/11_enum.md%26commit_hash%3D51de3c9d5a9074241f55c043b9951b384836b258)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/11_enum.md&commit_hash=51de3c9d5a9074241f55c043b9951b384836b258)

Set 生成的枚举类型。
枚举类型可以与类型规范一起使用，但可以通过将它们分类为类或定义修复程序来定义进一步的方法。

具有枚举类型的部分类型系统称为枚举部分类型。

```python
Bool = {True, False}
Status = {"ok", "error"}
```

由于 `1..7` 可以重写为 `{1, 2, 3, 4, 5, 6, 7}`，所以当元素是有限的时，Enum 类型本质上等同于 Range 类型。

```python
Binary! = Class {0, 1}!.
    invert! ref! self =
        if! self == 0:
            do!
                self.set! 1
            do!
                self.set! 0

b = Binary!.new !0
b.invert!()
```

顺便说一下，Erg 的 Enum 类型是一个包含其他语言中常见的枚举类型的概念。

```rust
// Rust
enum Status { Ok, Error }
```

```python
# Erg
Status = {"Ok", "Error"}
```

Rust 的不同之处在于它使用了结构子类型(SST)。

```rust
// Status 和 ExtraStatus 之间没有关系。
enum Status { Ok, Error }
enum ExtraStatus { Ok, Error, Unknown }

// 可实施的方法
impl Status {
    // ...
}
impl ExtraStatus {
    // ...
}
```

```python
# Status > ExtraStatus，Status的元素可以使用ExtraStatus的方法
Status = Trait {"Ok", "Error"}
    # ...
ExtraStatus = Trait {"Ok", "Error", "Unknown"}
    # ...
```

方法也可以通过补丁添加。

使用“或”运算符明确指示包含或向现有 Enum 类型添加选项。

```python
ExtraStatus = Status or {"Unknown"}
```

一个元素所属的所有类都相同的枚举类型称为同质枚举类型。

默认情况下，可以将需求类型为同类枚举类型的类视为元素所属类的子类。

如果您不想这样做，可以将其设为包装类

```python
Abc = Class {"A", "B", "C"}
Abc.new("A").is_uppercase()

OpaqueAbc = Class {inner = {"A", "B", "C"}}.
    new inner: {"A", "B", "C"} = Self.new {inner;}
OpaqueAbc.new("A").is_uppercase() # 类型错误
```

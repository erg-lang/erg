# 枚举类型

“枚举类型”（Enum type）由 Set 生成。虽然枚举类型可以保持不变，但可以通过类化或定义修补程序来定义其他方法。枚举部分类型系统称为枚举部分类型。


```erg
Bool = {True, False}
Status = {"ok", "error"}
```

因为被重写为<gtr=“10”/>，所以在元素有限的情况下，枚举类型和区间类型本质上是等价的。


```erg
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

顺便一提，Erg 的枚举类型是一个包含其他语言中常见的枚举类型的概念。


```rust
// Rust
enum Status { Ok, Error }
```


```erg
# Erg
Status = {"Ok", "Error"}
```

它与 Rust 的区别在于它采用了结构子类型 (SST)。


```rust
// StatusとExtraStatusの間にはなんの関係もない
enum Status { Ok, Error }
enum ExtraStatus { Ok, Error, Unknown }

// メソッドを実装可能
impl Status {
    // ...
}
impl ExtraStatus {
    // ...
}
```


```erg
# Status > ExtraStatusであり、Statusの要素はExtraStatusのメソッドを使える
Status = Trait {"Ok", "Error"}
    # ...
ExtraStatus = Trait {"Ok", "Error", "Unknown"}
    # ...
```

还可以使用 patching 添加方法。

如果要显式显示包含关系，或者要向现有 Enum 类型添加选项，请使用运算符。


```erg
ExtraStatus = Status or {"Unknown"}
```

元素所属的所有类都相同的枚举类型称为“等质”（homogenous）枚举类型。缺省情况下，要求类型相同的枚举类型的类可以被视为元素所属类的子类。如果你不想这样做，则最好是包装类。


```erg
Abc = Class {"A", "B", "C"}
Abc.new("A").is_uppercase()

OpaqueAbc = Class {inner = {"A", "B", "C"}}.
    new inner: {"A", "B", "C"} = Self.new {inner;}
OpaqueAbc.new("A").is_uppercase() # TypeError
```

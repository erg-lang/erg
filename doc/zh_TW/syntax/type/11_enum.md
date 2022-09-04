# 枚舉類型

“枚舉類型”（Enum type）由 Set 生成。雖然枚舉類型可以保持不變，但可以通過類化或定義修補程序來定義其他方法。枚舉部分類型系統稱為枚舉部分類型。


```erg
Bool = {True, False}
Status = {"ok", "error"}
```

因為被重寫為<gtr=“10”/>，所以在元素有限的情況下，枚舉類型和區間類型本質上是等價的。


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

順便一提，Erg 的枚舉類型是一個包含其他語言中常見的枚舉類型的概念。


```rust
// Rust
enum Status { Ok, Error }
```


```erg
# Erg
Status = {"Ok", "Error"}
```

它與 Rust 的區別在於它採用了結構子類型 (SST)。


```rust
// StatusとExtraStatusの間にはなんの関係もない
enum Status { Ok, Error }
enum ExtraStatus { Ok, Error, Unknown }

// メソッドを実裝可能
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

還可以使用 patching 添加方法。

如果要顯式顯示包含關係，或者要向現有 Enum 類型添加選項，請使用運算符。


```erg
ExtraStatus = Status or {"Unknown"}
```

元素所屬的所有類都相同的枚舉類型稱為“等質”（homogenous）枚舉類型。缺省情況下，要求類型相同的枚舉類型的類可以被視為元素所屬類的子類。如果你不想這樣做，則最好是包裝類。


```erg
Abc = Class {"A", "B", "C"}
Abc.new("A").is_uppercase()

OpaqueAbc = Class {inner = {"A", "B", "C"}}.
    new inner: {"A", "B", "C"} = Self.new {inner;}
OpaqueAbc.new("A").is_uppercase() # TypeError
```
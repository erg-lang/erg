# 枚舉類型

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/11_enum.md%26commit_hash%3Db713e6f5cf9570255ccf44d14166cb2a9984f55a)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/11_enum.md&commit_hash=b713e6f5cf9570255ccf44d14166cb2a9984f55a)

Set 生成的枚舉類型
枚舉類型可以與類型規范一起使用，但可以通過將它們分類為類或定義修復程序來定義進一步的方法

具有枚舉類型的部分類型系統稱為枚舉部分類型

```python
Bool = {True, False}
Status = {"ok", "error"}
```

由于 `1..7` 可以重寫為 `{1, 2, 3, 4, 5, 6, 7}`，所以當元素是有限的時，Enum 類型本質上等同于 Range 類型

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

順便說一下，Erg 的 Enum 類型是一個包含其他語言中常見的枚舉類型的概念

```rust
// Rust
enum Status { Ok, Error }
```

```python
# Erg
Status = {"Ok", "Error"}
```

Rust 的不同之處在于它使用了結構子類型(SST)

```rust
// Status 和 ExtraStatus 之間沒有關系
enum Status { Ok, Error }
enum ExtraStatus { Ok, Error, Unknown }

// 可實施的方法
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

方法也可以通過補丁添加

使用"或"運算符明確指示包含或向現有 Enum 類型添加選項

```python
ExtraStatus = Status or {"Unknown"}
```

一個元素所屬的所有類都相同的枚舉類型稱為同質枚舉類型

默認情況下，可以將需求類型為同類枚舉類型的類視為元素所屬類的子類

如果您不想這樣做，可以將其設為包裝類

```python
Abc = Class {"A", "B", "C"}
Abc.new("A").is_uppercase()

OpaqueAbc = Class {inner = {"A", "B", "C"}}.
    new inner: {"A", "B", "C"} = Self.new {inner;}
OpaqueAbc.new("A").is_uppercase() # 類型錯誤
```
<p align='center'>
    <a href='./10_interval.md'>上一頁</a> | <a href='./12_refinement.md'>下一頁</a>
</p>
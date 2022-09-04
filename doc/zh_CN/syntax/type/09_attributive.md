# 属性类型

属性类型是包含 Record 和 Dataclass、Patch、Module 等的类型。
属于属性类型的类型不是值类型。

## 记录类型复合

可以展平复合的记录类型。
例如，`{... {.name = Str; .age = Nat}; ... {.name = Str; .id = Nat}}` 变成 `{.name = Str; .age = 自然； .id = Nat}`。
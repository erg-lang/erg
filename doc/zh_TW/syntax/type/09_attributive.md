# 屬性類型

屬性類型是包含 Record 和 Dataclass、Patch、Module 等的類型。
屬于屬性類型的類型不是值類型。

## 記錄類型復合

可以展平復合的記錄類型。
例如，`{... {.name = Str; .age = Nat}; ... {.name = Str; .id = Nat}}` 變成 `{.name = Str; .age = 自然； .id = Nat}`。
# 属性类型

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/09_attributive.md%26commit_hash%3D412a6fd1ea507a7afa1304bcef642dfe6b3a0872)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/09_attributive.md&commit_hash=412a6fd1ea507a7afa1304bcef642dfe6b3a0872)

属性类型是包含 Record 和 Dataclass、Patch、Module 等的类型
属于属性类型的类型不是值类型

## 记录类型复合

可以展平复合的记录类型
例如，`{... {.name = Str; .age = Nat}; ... {.name = Str; .id = Nat}}` 变成 `{.name = Str; .age = 自然； .id = Nat}`

<p align='center'>
    <a href='./08_value.md'>上一页</a> | <a href='./10_interval.md'>下一页</a>
</p>
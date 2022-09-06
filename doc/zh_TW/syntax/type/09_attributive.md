# 屬性類型

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/09_attributive.md%26commit_hash%3Deccd113c1512076c367fb87ea73406f91ff83ba7)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/09_attributive.md&commit_hash=eccd113c1512076c367fb87ea73406f91ff83ba7)

屬性類型是包含 Record 和 Dataclass、Patch、Module 等的類型。
屬于屬性類型的類型不是值類型。

## 記錄類型復合

可以展平復合的記錄類型。
例如，`{... {.name = Str; .age = Nat}; ... {.name = Str; .id = Nat}}` 變成 `{.name = Str; .age = 自然； .id = Nat}`。
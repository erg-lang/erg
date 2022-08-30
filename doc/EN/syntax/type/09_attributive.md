# Attributive Type

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/09_attributive.md%26commit_hash%3Dae6d00168c17428bf967e44db3e6360e2471df8b)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/09_attributive.md&commit_hash=ae6d00168c17428bf967e44db3e6360e2471df8b)

Attribute types are types that contain Record and Dataclass, Patch, Module, etc.
Types belonging to attribute types are not value types.

## Record Type Composite 

It is possible to flatten Record types composited.
For example, `{... {.name = Str; .age = Nat}; ... {.name = Str; .id = Nat}}` becomes `{.name = Str; .age = Nat; .id = Nat}`.
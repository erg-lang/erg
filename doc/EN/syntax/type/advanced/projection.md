# Projection Type

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/advanced/projection.md%26commit_hash%3D317b5973c354984891523d14a5e6e8f1cc3923ec)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced/projection.md&commit_hash=317b5973c354984891523d14a5e6e8f1cc3923ec)

A projection type represents a type such as ``Self.AddO`` in the following code.

```erg
Add R = Trait {
    . `_+_` = Self, R -> Self.AddO
    .AddO = Type
}

AddForInt = Patch(Int, Impl: Add Int)
AddForInt.
    AddO = Int
```

The type ``Add(R)`` can be said to be a type that defines addition with some object. Since the method should be a type attribute, the `+` type declaration should be written below the indentation.
The mise-en-scène of the `Add` type is the declaration `.AddO = Type`, and the entity of the `.AddO` type, which is a projective type, is held by a type that is a subtype of `Add`. For example, `Int.AddO = Int`, `Odd.AddO = Even`.

```erg
assert Int < Add
assert Int.AddO == Int
assert Odd < Add
assert Odd.AddO == Even
```

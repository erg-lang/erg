# Projection Type

A projection type represents a type such as ``Self.AddO`` in the following code.

```python
Add R = Trait {
    . `_+_` = Self, R -> Self.AddO
    .AddO = Type
}

AddForInt = Patch(Int, Impl := Add Int)
AddForInt.
    AddO = Int
```

The type ``Add(R)`` can be said to be a type that defines addition with some object. Since the method should be a type attribute, the `+` type declaration should be written below the indentation.
The mise-en-scène of the `Add` type is the declaration `.AddO = Type`, and the entity of the `.AddO` type, which is a projective type, is held by a type that is a subtype of `Add`. For example, `Int.AddO = Int`, `Odd.AddO = Even`.

```python
assert Int < Add
assert Int.AddO == Int
assert Odd < Add
assert Odd.AddO == Even
```

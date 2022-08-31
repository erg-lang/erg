# Marker Trait

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/advanced/marker_trait.md%26commit_hash%3D317b5973c354984891523d14a5e6e8f1cc3923ec)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced/marker_trait.md&commit_hash=317b5973c354984891523d14a5e6e8f1cc3923ec)

A marker trait is a trait with no required attributes. That is, it can be Impl without implementing a method.
It may seem meaningless without the required attribute, but it registers the information that it belongs to that trait, so that patch methods can be used and the compiler can give it special treatment.

All marker traits are encompassed by the `Marker` trait.
The `Light` provided in the standard is a kind of `marker` trace.

```erg
Light = Subsume Marker
```

```erg
Person = Class {.name = Str; .age = Nat} and Light
```

```erg
M = Subsume Marker

MarkedInt = Inherit Int, Impl: M

i = MarkedInt.new(2)
assert i + 1 == 2
assert i in M
```

The marker class can also be removed with the `Excluding` argument.

```erg
NInt = Inherit MarkedInt, Impl: N, Excluding: M
```

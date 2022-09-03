# Marker Trait

A marker trait is a trait with no required attributes. That is, it can be Impl without implementing a method.
It may seem meaningless without the required attribute, but it registers the information that it belongs to that trait, so that patch methods can be used and the compiler can give it special treatment.

All marker traits are encompassed by the `Marker` trait.
The `Light` provided in the standard is a kind of marker trait.

```erg
Light = Subsume Marker
```

```erg
Person = Class {.name = Str; .age = Nat} and Light
```

```erg
M = Subsume Marker

MarkedInt = Inherit Int, Impl := M

i = MarkedInt.new(2)
assert i + 1 == 2
assert i in M
```

The marker class can also be removed with the `Excluding` argument.

```erg
NInt = Inherit MarkedInt, Impl := N, Excluding: M
```

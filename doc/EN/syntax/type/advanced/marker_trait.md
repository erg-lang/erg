# Marker Trait

Marker traits are traits without required attributes. That is, you can Impl without implementing any method.
It seems meaningless without the required attribute, but since the information that it belongs to the trait is registered, you can use the patch method or get special treatment by the compiler.

All marker traits are subsumed by the `Marker` trait.
`Light` provided as standard is a kind of marker trait.

``` erg
Light = Subsume Marker
```

``` erg
Person = Class {.name = Str; .age = Nat} and Light
```

``` erg
M = Subsume Marker

MarkedInt = Inherit Int, Impl := M

i = MarkedInt.new(2)
assert i + 1 == 2
assert i in M
```

Marker classes can also be excluded with the `Excluding` argument.

``` erg
NInt = Inherit MarkedInt, Impl := N, Excluding: M
```
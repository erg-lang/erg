# Trait

Traits are nominal types that add a type attribute requirement to record types.
It is similar to the Abstract Base Class (ABC) in Python, but it has the feature of being able to perform algebraic operations.

Traits are used when you want to identify different classes. Examples of builtin traits are `Eq` and `Add`.
`Eq` requires `==` to be implemented. `Add` requires the implementation of `+` (in-place).

So any class that implements these can be (partially) identified as a subtype of trait.

As an example, let's define a `Norm` trait that computes the norm (length) of a vector.

```python
Norm = Trait {.norm = (self: Self) -> Int}
```

Note that traits can only be declared, not implemented.
Traits can be "implemented" for a class as follows:

```python
Point2D = Class {.x = Int; .y = Int}
Point2D|<: Norm|.
    Norm self = self.x**2 + self.y**2

Point3D = Class {.x = Int; .y = Int; .z = Int}
Point3D|<: Norm|.
    norm self = self.x**2 + self.y**2 + self.z**2
```

Since `Point2D` and `Point3D` implement `Norm`, they can be identified as types with the `.norm` method.

```python
norm x: Norm = x.norm()

assert norm(Point2D.new({x = 1; y = 2})) == 5
assert norm(Point3D.new({x = 1; y = 2; z = 3})) == 14
```

Error if the required attributes are not implemented.

```python,compile_fail
Point3D = Class {.x = Int; .y = Int; .z = Int}

Point3D|<: Norm|.
    foo self = 1
```

One of the nice things about traits is that you can define methods on them in Patch (described later).

```python
@Attach NotEqual
Eq = Trait {. `==` = (self: Self, other: Self) -> Bool}

NotEq = Patch Eq
NotEq.
    `! =` self, other = not self.`==` other
```

With the `NotEq` patch, all classes that implement `Eq` will automatically implement `!=`.

## Trait operations

Traits, like structural types, can apply operations such as composition, substitution, and elimination (e.g. `T and U`). The resulting trait is called an instant trait.

```python
T = Trait {.x = Int}
U = Trait {.y = Int}
V = Trait {.x = Int; y: Int}
assert Structural(T and U) == Structural V
assert Structural(V not U) == Structural T
W = Trait {.x = Ratio}
assert Structural(W) ! = Structural(T)
assert Structural(W) == Structural(T.replace {.x = Ratio})
```

Trait is also a type, so it can be used for normal type specification.

```python
points: [Norm; 2] = [Point2D::new(1, 2), Point2D::new(3, 4)]
assert points.iter().map(x -> x.norm()).collect(Array) == [5, 25].
```

## Trait inclusion

`Subsume` allows you to define a trait that contains a certain trait as a supertype. This is called the __subsumption__ of a trait.
In the example below, `BinAddSub` subsumes `BinAdd` and `BinSub`.
This corresponds to Inheritance in a class, but unlike Inheritance, multiple base types can be combined using `and`. Traits that are partially excluded by `not` are also allowed.

```python
Add R = Trait {
    .AddO = Type
    . `_+_` = Self.(R) -> Self.AddO
}

Sub R = Trait {
    .SubO = Type
    . `_-_` = Self.(R) -> Self.SubO
}

BinAddSub = Subsume Add(Self) and Sub(Self)
```

## Structural Traits

Traits can be structured.

```python
SAdd = Structural Trait {
    . `_+_` = Self.(Self) -> Self
}
# |A <: SAdd| cannot be omitted
add|A <: SAdd| x, y: A = x.`_+_` y

C = Class {i = Int}
C.
    new i = Self.__new__ {i;}
    `_+_` self, other: Self = Self.new {i = self::i + other::i}

assert add(C.new(1), C.new(2)) == C.new(3)
```

Nominal traits cannot be used simply by implementing a request method, but must be explicitly declared to have been implemented.
In the following example, `add` cannot be used with an argument of type `C` because there is no explicit declaration of implementation. It must be `C = Class {i = Int}, Impl := Add`.

```python
Add = Trait {
    .`_+_` = Self.(Self) -> Self
}
# |A <: Add| can be omitted
add|A <: Add| x, y: A = x.`_+_` y

C = Class {i = Int}
C.
    new i = Self.__new__ {i;}
    `_+_` self, other: Self = Self.new {i = self::i + other::i}

add C.new(1), C.new(2) # TypeError: C is not a subclass of Add
# hint: inherit or patch 'Add'
```

Structural traits do not need to be declared for this implementation, but instead type inference does not work. Type specification is required for use.

## Polymorphic Traits

Traits can take parameters. This is the same as for polymorphic types.

```python
Mapper T: Type = Trait {
    .mapIter = {Iterator}
    .map = (self: Self, T -> U) -> Self.MapIter U
}

# ArrayIterator <: Mapper
# ArrayIterator.MapIter == ArrayMapper
# [1, 2, 3].iter(): ArrayIterator Int
# [1, 2, 3].iter().map(x -> "\{x}"): ArrayMapper Str
assert [1, 2, 3].iter().map(x -> "\{x}").collect(Array) == ["1", "2", "3"].
```

## Override in Trait

Derived traits can override the type definitions of the base trait.
In this case, the type of the overriding method must be a subtype of the base method type.

```python
# `Self.(R) -> O` is a subtype of ``Self.(R) -> O or Panic
Div R, O: Type = Trait {
    . `/` = Self.(R) -> O or Panic
}
SafeDiv R, O = Subsume Div, {
    @Override
    . `/` = Self.(R) -> O
}
```

## Implementing and resolving duplicate traits in the API

The actual definitions of `Add`, `Sub`, and `Mul` look like this.

```python
Add R = Trait {
    .Output = Type
    . `_+_` = Self.(R) -> .Output
}
Sub R = Trait {
    .Output = Type
    . `_-_` = Self.(R) -> .Output
}
Mul R = Trait {
    .Output = Type
    . `*` = Self.(R) -> .Output
}
```

`.Output` is duplicated. If you want to implement these multiple traits at the same time, specify the following.

```python
P = Class {.x = Int; .y = Int}
# P|Self <: Add(P)| can be abbreviated to P|<: Add(P)|
P|Self <: Add(P)|.
    Output = P
    `_+_` self, other = P.new {.x = self.x + other.x; .y = self.y + other.y}
P|Self <: Mul(Int)|.
    Output = P
    `*` self, other = P.new {.x = self.x * other; .y = self.y * other}
```

Duplicate APIs implemented in this way are almost always type inferred when used, but can also be resolved by explicitly specifying the type with `||`.

```python
print! P.Output # TypeError: ambiguous type
print! P|<: Mul(Int)|.Output # <class 'P'>
```

## Appendix: Differences from Rust traits

Erg's trait is faithful to the one proposed by [Schärli et al.](https://www.ptidej.net/courses/ift6251/fall06/presentations/061122/061122.doc.pdf).
In order to allow algebraic operations, traits are designed to be unable to have method implementations directory, but can be patched if necessary.

<p align='center'>
    <a href='./02_basic.md'>Previous</a> | <a href='./04_class.md'>Next</a>
</p>

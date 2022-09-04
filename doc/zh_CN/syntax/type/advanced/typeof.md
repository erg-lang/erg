# Typeof, classof

`Typeof` is a function that can peek into Erg's type inference system, and its behavior is complex.

``` erg
assert Typeof(1) == {I: Int | I == 1}
i: 1..3 or 5..10 = ...
assert Typeof(i) == {I: Int | (I >= 1 and I <= 3) or (I >= 5 and I <= 10)}

C = Class {i = Int}
I = C. new {i = 1}
assert Typeof(I) == {X: C | X == I}
J: C = ...
assert Typeof(J) == {i = Int}

assert {X: C | X == I} < C and C <= {i = Int}
```

The `Typeof` function returns the derived type, not the class of the object.
So for instance `I: C` of class `C = Class T`, `Typeof(I) == T`.
A value class does not have a corresponding record type. To solve this problem, value classes are supposed to be record types that have a `__valueclass_tag__` attribute.
Note that you cannot access this attribute, nor can you define a `__valueclass_tag__` attribute on a user-defined type.

``` erg
i: Int = ...
assert Typeof(i) == {__valueclass_tag__ = Phantom Int}
s: Str = ...
assert Typeof(s) == {__valueclass_tag__ = Phantom Str}
```

`Typeof` outputs only structured types. I explained that structured types include attribute types, sieve types, and (true) algebraic types.
These are independent types (inference precedence exists) and inference conflicts do not occur.
Attribute types and algebraic types can span multiple classes, while sieve types are subtypes of a single class.
Erg infers object types as sieve types as much as possible, and when that is not possible, expands sieve base classes to structured types (see below).

## structured

All classes can be converted to derived types. This is called __structuring__. The structured type of a class can be obtained with the `Structure` function.
If a class is defined with `C = Class T` (all classes are defined in this form) then `Structure(C) == T`.

``` erg
C = Class {i = Int}
assert Structure(C) == {i = Int}
D = Inherit C
assert Structure(D) == {i = Int}
Nat = Class {I: Int | I >= 0}
assert Structure(Nat) == {I: Int | I >= 0}
Option T = Class (T or NoneType)
assert Structure(Option Int) == Or(Int, NoneType)
assert Structure(Option) # TypeError: only monomorphized types can be structured
# You can't actually define a record with __valueclass_tag__, but conceptually
assert Structure(Int) == {__valueclass_tag__ = Phantom Int}
assert Structure(Str) == {__valueclass_tag__ = Phantom Str}
assert Structure((Nat, Nat)) == {__valueclass_tag__ = Phantom(Tuple(Nat, Nat))}
assert Structure(Nat -> Nat) == {__valueclass_tag__ = Phantom(Func(Nat, Nat))}
# Marker classes are also record types with __valueclass_tag__
M = Inherit Marker
assert Structure(M) == {__valueclass_tag__ = Phantom M}
D = Inherit(C and M)
assert Structure(D) == {i = Int; __valueclass_tag__ = Phantom M}
E = Inherit(Int and M)
assert Structure(E) == {__valueclass_tag__ = Phantom(And(Int, M))}
F = Inherit(E not M)
assert Structure(F) == {__valueclass_tag__ = Phantom Int}
```
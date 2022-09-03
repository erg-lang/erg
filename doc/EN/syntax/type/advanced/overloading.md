# Overloading

Erg does not support __ad hoc polymorphism__. That is, multiple definitions of functions and Kinds (overloading) are not possible. However, you can reproduce the overloading behavior by using a combination of a trait and a patch.
You can use traits instead of trait classes, but then all types that implement `.add1` will be covered.

```erg
Add1 = Trait {
    .add1: Self.() -> Self
}
IntAdd1 = Patch Int, Impl := Add1
IntAdd1.
    add1 self = self + 1
RatioAdd1 = Patch Ratio, Impl := Add1
RatioAdd1.
    add1 self = self + 1.0

add1|X <: Add1| x: X = x.add1()
assert add1(1) == 2
assert add1(1.0) == 2.0
```

Such a polymorphism by accepting all subtypes of a type is called __subtyping polymorphism__.

If the process is exactly the same for each type, it can be written as below. The above is used when the behavior changes from class to class (but the return type is the same).
A polymorphism that uses type arguments is called __parametric polymorphism__. Parametric polymorphism is often used in conjunction with subtyping, as shown below, in which case it is a combination of parametric and subtyping polymorphism.

```erg
add1|T <: Int or Str| x: T = x + 1
assert add1(1) == 2
assert add1(1.0) == 2.0
```

Also, overloading of types with different numbers of arguments can be reproduced with default arguments.

```erg
C = Class {.x = Int; .y = Int}
C.
    new(x, y := 0) = Self::__new__ {.x; .y}

assert C.new(0, 0) == C.new(0)
```

Erg takes the stance that you cannot define a function that behaves completely differently, such as having a different type depending on the number of arguments, but if the behavior is different to begin with, it should be named differently.

In conclusion, Erg prohibits overloading and adopts subtyping plus parametric polymorphism for the following reasons.

First, overloaded functions are distributed in their definitions. This makes it difficult to report the cause of an error when it occurs.
Also, importing a subroutine may change the behavior of already defined subroutines.

```erg
{id; ...} = import "foo"
...
id x: Int = x
...
id x: Ratio = x
...
id "str" # TypeError: id is not implemented for Str
# But... But... where did this error come from?
```

Second, it is incompatible with default arguments. When a function with default arguments is overloaded, there is a problem with which one takes precedence.

```erg
f x: Int = ...
f(x: Int, y := 0) = ...

f(1) # which is chosen?
```

Furthermore, it is incompatible with the declaration.
The declaration `f: Num -> Num` cannot specify which definition it refers to. This is because `Int -> Ratio` and `Ratio -> Int` are not inclusive.

```erg
f: Num -> Num
f(x: Int): Ratio = ...
f(x: Ratio): Int = ...
```

And the grammar is inconsistent: Erg prohibits variable reassignment, but the overloaded grammar looks like reassignment.
Nor can it be replaced by an anonymous function.

```erg
# same as `f = x -> body`
f x = body

# same as... what?
f x: Int = x
f x: Ratio = x
```

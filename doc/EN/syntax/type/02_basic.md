# Basic syntax for types

## Type specification

In Erg, the type of a variable can be specified after `:` as follows. This can be done at the same time as an assignment.

```python
i: Int # Declare the variable i to be of type Int
i: Int = 1
j = 1 # type specification can be omitted
```

For simple variable assignments, most type specifications can be omitted.
Type specifications are more useful when defining subroutines and types.

```python
# Type specification for parameters
f x, y: Array Int = ...
T X, Y: Array Int = ...
```

Note that in the above case, `x, y` are both `Array Int`.

```python
# The value of a capital variable must be a constant expression
f X: Int = X
```

Alternatively, if you don't need complete information about the type argument, you can omit it with `_`.

```python
g v: [T; _] = ...
```

Note, however, `_` at a type specification implies `Object`.

```python,compile_fail
f x: _, y: Int = x + y # TypeError: + is not implemented between Object and Int
```

## Type ascription

Erg can explicitly indicate the type of any expression as well as variables. This syntax is called type ascription.

Variable type ascription = type specification.

```python
x = 1: Nat
f("a": Str)
f("a"): Int
"a": Nat # TypeError:
```

## Subtype specification

In addition to the `:` (type declaration operator), Erg also allows you to specify the relationship between types by using `<:` (partial type declaration operator).
The left side of `<:` can only specify a class. Use `Subtypeof` or similar operators to compare structural types.

This is also often used when defining subroutines or types, rather than simply specifying variables.

```python
# Subtype specification of an argument
f X <: T = ...

# Subtype specification of the required attribute (.Iterator attribute is required to be a subtype of type Iterator)
Iterable T = Trait {
    .Iterator = {Iterator} # {Iterator} == {I: Type | I <: Iterator}
    .iter = Self.() -> Self.Iterator T
    ...
}
```

You can also use a subtype specification when defining a class to statically check whether the class is a subtype of the specified type.

```python
# Class C is a subtype of Show
C = Class Object, Impl := Show
C.show self = ... # Show's required attributes.
```

You can also specify a subtype only in specific cases.

```python
K T: Eq
K Int <: Show and Eq
K T = Class Object
K(T).
    `==` self, other = ...
K(Int).
    show self = ...
```

Subtype specification is recommended when implementing structural types.
This is because, due to the nature of structural subtyping, typo or type specification errors will not cause errors when implementing required attributes.

```python
C = Class Object
C.shoe self = ... # Show is not implemented due to Typo (it is considered just a unique method).
```

## Attribute definitions

Attributes can be defined for traits and classes only in modules.

```python
C = Class()
C.pub_attr = "this is public"
C::private_attr = "this is private"

c = C.new()
assert c.pub_attr == "this is public"
```

The syntax for defining a batch definition is called a batch definition, in which a newline is added after `C.` or `C::` and the definitions are grouped together below the indentation.

```python
C = Class()
C.pub1 = ...
C.pub2 = ...
C::priv1 = ...
C::priv2 = ...
# is equivalent to
C = Class()
C.
    pub1 = ...
    C. pub2 = ...
C::
    priv1 = ...
    priv2 = ...
```

## Aliasing

Types can be aliased. This allows long types, such as record types, to be shortened.

```python
Id = Int
Point3D = {x = Int; y = Int; z = Int}
IorS = Int or Str
Vector = Array Int
```

Also, when displaying errors, the compiler will use aliases for composite types (in the above example, right-hand-side types other than the first) if they are defined.

However, only one alias of the same type is allowed per module, and multiple aliases will result in a warning.
This means that types with different purposes should be defined as separate types.
The purpose is also to prevent adding aliases on top of types that already have aliases.

```python,compile_warn
Id = Int
UserId = Int # TypeWarning: duplicate aliases: Id and UserId

Ids = Array Id
Ints = Array Int # TypeWarning: duplicate aliases: Isd and Ints

IorS = Int or Str
IorSorB = IorS or Bool
IorSorB_ = Int or Str or Bool # TypeWarning: duplicate aliases: IorSorB and IorSorB_

Point2D = {x = Int; y = Int}
Point3D = {.... Point2D; z = Int}
Point = {x = Int; y = Int; z = Int} # TypeWarning: duplicate aliases: Point3D and Point
```

<p align='center'>
    <a href='./01_type_system.md'>Previous</a> | <a href='./03_trait.md'>Next</a>
</p>

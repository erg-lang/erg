# Erg's Type System

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/01_type_system.md%26commit_hash%3D417bfcea08ed0e09f715f5d272842510fca8f6dd)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/01_type_system.md&commit_hash=417bfcea08ed0e09f715f5d272842510fca8f6dd)

The following is a brief description of Erg's type system. Details are explained in other sections.

## How to define

One of the unique features of Erg is that there is not much difference in syntax between (normal) variable, function (subroutine), and type (Kind) definitions. All are defined according to the syntax of normal variable and function definitions.

```erg
f i: Int = i + 1
f # <function f>
f(1) # 2
f.method self = ... # SyntaxError: cannot define a method to a subroutine

T I: Int = {...}
T # <kind 'T'>
T(1) # Type T(1)
T.method self = ...
D = Class {private = Int; .public = Int}
D # <class 'D'>
o1 = {private = 1; .public = 2} # o1 is an object that does not belong to any class
o2 = D.new {private = 1; .public = 2} # o2 is an instance of D
o2 = D.new {.public = 2} # InitializationError: class 'D' requires attribute 'private'(: Int) but not defined
```

## Classification

All objects in Erg are strongly typed.
The top-level type is `{=}`, which implements `__repr__`, `__hash__`, `clone`, etc. (not required methods, and these attributes cannot be overridden).
Erg's type system incorporates structural subtyping (SST). The types typed by this system are called Structural types.
There are three major types of structural types: Attributive (attribute type), Refinement (refinement type), and Algebraic (algebraic type).

|           | Record      | Enum       | Interval       | Union       | Intersection | Diff         |
| --------- | ----------- | ---------- | -------------- | ----------- | ------------ | ------------ |
| kind      | Attributive | Refinement | Refinement     | Algebraic   | Algebraic    | Algebraic    |
| generator | record      | set        | range operator | or operator | and operator | not operator |

Nominal subtyping (NST) can also be used, and the conversion of an SST type to an NST type is called nominalization of the type. The resulting type is called a nominal type.
In Erg, the nominal types are classes and traits. When we simply say class/trait, we often mean record class/trait.

|     | Type           | Abstraction      | Subtyping procedure |
| --- | -------------- | ---------------- | ------------------- |
| NST | NominalType    | Trait            | Inheritance         |
| SST | StructuralType | Structural Trait | (Implicit)          |

The type for the entire nominal type (`NominalType`) and the type for the entire structural type (`StructuralType`) are subtypes of the type for the entire type (`Type`).

Erg can pass arguments (type arguments) to the type definition. An `Option`, `Array`, etc. with type arguments are called a polynomial kind. These are not themselves types, but they become types by applying arguments. Types such as `Int`, `Str`, etc., which have no arguments, are called simple types (scalar types).

A type can be regarded as a set, and there is an inclusion relation. For example, `Num` contains `Add`, `Sub`, etc., and `Int` contains `Nat`.
The upper class of all classes is `Object == Class {:}` and the lower class of all types is `Never == Class {}`. This is described below.

## Types

A type like `Array T` can be regarded as a function of type `Type -> Type` that takes type `T` as an argument and returns type `Array T` (also called Kind in type theory). Types like `Array T` are specifically called polymorphic types, and `Array` itself is called unary Kind.

The type of a function whose argument and return types are known is denoted as `(T, U) -> V`. If you want to specify an entire two-argument function of the same type, you can use `|T| (T, T) -> T`, and if you want to specify an entire N-argument function, you can use `Func N`. However, the `Func N` type has no information about the number of arguments or their types, so all return values are of type `Obj` when called.

The `Proc` type is denoted as `() => Int` and so on. Also, the name of the `Proc` type instance must end with `!` at the end.

A `Method` type is a function/procedure whose first argument is the object `self` to which it belongs (by reference). For dependent types, you can also specify the type of yourself after the method is applied. This is `T!(!N)` type and `T!(N ~> N-1). () => Int` and so on.

Erg's array (Array) is what Python calls a list. `[Int; 3]` is an array class that contains three objects of type `Int`.

> __Note__: `(Type; N)` is both a type and a value, so it can be used like this.
>
> ```erg.
> Types = (Int, Str, Bool)
>
> for! Types, T =>
>     print! T
> # Int Str Bool
> a: Types = (1, "aaa", True)
> ```

```erg
pop|T, N|(l: [T; N]): ([T; N-1], T) =
    [...l, last] = l
    (l, last)

lpop|T, N|(l: [T; N]): (T, [T; N-1]) =
    [first, ...l] = l
    (first, l)
```

A type ends with `!` can be rewritten internal structure. For example, the `[T; !N]` class is a dynamic array.
To create an object of type `T!` from an object of type `T`, use the unary operator `!`.

```erg
i: Int! = !1
i.update! i -> i + 1
assert i == 2
arr = [1, 2, 3]
arr.push! 4 # ImplError:
mut_arr = [1, 2, 3].into [Int; !3]
mut_arr.push4
assert mut_arr == [1, 2, 3, 4].
```

## Type Definitions

Types are defined as follows.

```erg
Point2D = {.x = Int; .y = Int}
```

Note that if `.` is omitted from a variable, it becomes a private variable used within the type. However, this is also a required attribute.
Since types are also objects, there are attributes on the types themselves. Such attributes are called type attributes. In the case of a class, they are also called class attributes.

## Data type

As mentioned earlier, a "type" in Erg roughly means a set of objects.

The following is a definition of the `Add` type, which requires `+` (the middle operator). `R, O` are the so-called type parameters, which can be a true type (class) such as `Int` or `Str`. In other languages, type parameters are given a special notation (generics, templates, etc.), but in Erg they can be defined just like normal parameters.
Type parameters can also be used for types other than type objects. For example, the array type `[Int; 3]` is a syntax sugar for `Array Int, 3`. If the type implementations overlap, the user must explicitly choose one.

```erg
Add R = Trait {
    .AddO = Type
    . `_+_` = Self.(R) -> Self.AddO
}
```

.`_+_` is an abbreviation for Add.`_+_`. The prefix operator .`+_` is a method of type `Num`.

```erg
Num = Add and Sub and Mul and Eq
NumImpl = Patch Num
NumImpl.
    `+_`(self): Self = self
    ...
```

Polymorphic types can be treated like functions. They can be monomorphic by specifying them as `Mul Int, Str`, etc. (in many cases, they are inferred with real arguments without specifying them).

```erg
1 + 1
`_+_` 1, 1
Nat.`_+_` 1, 1
Int.`_+_` 1, 1
```

The top four lines return the same result (to be exact, the bottom one returns `Int`), but it is common to use the top one.
```Ratio.`_+_`(1, 1)``` will return `2.0` without error.
This is because `Int <: Ratio`, so `1` is downcast to `Ratio`.
But this is not cast.

```erg
i = 1
if i: # TypeError: i: Int cannot be cast to Bool, use Int.is_zero() instead.
    log "a"
    log "b"
```

This is because `Bool <: Int` (`True == 1`, `False == 0`). Casts to subtypes generally require validation.

## Type Inference System

Erg uses static duck typing, so there is little need to explicitly specify the type.

```erg
f x, y = x + y
```

In the case of the code above, the type with `+`, i.e., `Add` is automatically inferred; Erg first infers the smallest type. If `f 0, 1`, it will infer `f x: {0}, y: {1}`, if `n: Nat; f n, 1`, it will infer `f x: Nat, y: {1}`. After minimization, the type is increased until an implementation is found. In the case of `{0}, {1}`, `Nat` is monomorphic to `Nat` since `Nat` is the smallest type with a `+` implementation.
If `{0}, {-1}`, it is monomorphic to `Int` since it does not match `Nat`. If there is no relationship between subtypes and supertypes, the one with the lowest concentration (number of instances) (or even fewer arguments in the case of polymorphic types) is tried first.
`{0}` and `{1}` are enumerated types that are partial types such as `Int` and `Nat`.
Enumerated types, for example, can be given names and request/implementation methods. In namespaces that have access to that type, objects that satisfy the request can use the implementation method.

```erg
Binary = Patch {0, 1}
Binary.
    # self contains an instance. In this example, either 0 or 1.
    # If you want to rewrite self, you must append ! must be added to the type name and method name.
    is_zero(self) = match self:
        0 -> True
        1 -> False # You can also use _ -> False
    is_one(self) = not self.is_zero()
    to_bool(self) = match self:
        0 -> False
        1 -> True
```

Thereafter, the code `0.to_bool()` is possible (although `0 as Bool == False` is defined built-in).
Here is an example of a type that can actually rewrite `self` as shown in the code.

```erg
Binary! = Patch {0, 1}!
Binary!
    switch! ref! self = match! self:
        0 => self = 1
        1 => self = 0

b = !1
b.switch!()
print! b # => 0
```

## Structure type (anonymous type)

```erg
Binary = {0, 1}
```

`Binary` in the above code is a type whose elements are `0` and `1`. It is also a subtype of the `Int` type, which has both `0` and `1`.
An object like `{}` is itself a type and can be used with or without assignment to a variable as above.
Such types are called structural types. When we want to emphasize its use as the latter in contrast to a class (named type), it is also called an unnamed type. A structural type such as `{0, 1}` is called an enumerated type, and there are also interval types, record types, and so on.

### Type Identity

The following cannot be specified. For example, you cannot specify `Int` and `Int` and `Int` and `Int` and `Int` and `Int`.
For example, `Int` and `Str` are both `Add`, but `Int` and `Str` cannot be added.

```erg
add l: Add, r: Add =
    l + r # TypeError: there is no implementation of `_+_`: |T, U <: Add| (T, U) -> <Failure>
```

Also, the types `A` and `B` below are not considered the same type. However, the type `O` is considered to match.

```erg
... |R1; R2; O; A <: Add(R1, O); B <: Add(R2, O)|
```

<p align='center'>
    Previous | <a href='./02_basic.md'>Next</a>
</p>

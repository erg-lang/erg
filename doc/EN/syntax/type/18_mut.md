# Mutable Type

> __Warning__: The information in this section is old and contains some errors.

By default all types in Erg are immutable, i.e. their internal state cannot be updated.
But you can of course also define mutable types. Variable types are declared with `!`.

```python
Person! = Class({name = Str; age = Nat!})
Person!.
    greet! ref! self = print! "Hello, my name is {self::name}. I am {self::age}."
    inc_age!ref!self = self::name.update!old -> old + 1
```

To be precise, a type whose base type is a mutable type, or a composite type containing mutable types, must have a `!` at the end of the type name. Types without `!` can exist in the same namespace and are treated as separate types.
In the example above, the `.age` attribute is mutable and the `.name` attribute is immutable. If even one attribute is mutable, the whole is mutable.

Mutable types can define procedural methods that rewrite instances, but having procedural methods does not necessarily make them mutable. For example, the array type `[T; N]` implements a `sample!` method that randomly selects an element, but of course does not destructively modify the array.

Destructive operations on mutable objects are primarily done via the `.update!` method. The `.update!` method is a higher-order procedure that updates `self` by applying the function `f`.

```python
i = !1
i.update! old -> old + 1
assert i == 2
```

The `.set!` method simply discards the old content and replaces it with the new value. .set!x = .update!_ -> x.

```python
i = !1
i.set! 2
assert i == 2
```

The `.freeze_map` method operates on values ​​unchanged.

```python
a = [1, 2, 3].into [Nat; !3]
x = a.freeze_map a: [Nat; 3] -> a.iter().map(i -> i + 1).filter(i -> i % 2 == 0).collect(Array)
```

In a polymorphic immutable type the type argument `T` of the type is implicitly assumed to be immutable.

```python
# ImmutType < Type
KT: ImmutType = Class ...
K!T: Type = Class ...
```

In the standard library, variable `(...)!` types are often based on immutable `(...)` types. However, `T!` and `T` types have no special linguistic relationship, and may not be constructed as such [<sup id="f1">1</sup>](#1) .

Note that there are several types of object mutability.
Below we will review the immutable/mutable semantics of the built-in collection types.

```python
# array type
## immutable types
[T; N] # Cannot perform mutable operations
## mutable types
[T!; N] # can change contents one by one
[T; !N] # variable length, content is immutable but can be modified by adding/deleting elements
[!T; N] # The content is an immutable object, but it can be replaced by a different type (actually replaceable by not changing the type)
[!T; !N] # type and length can be changed
[T!; !N] # content and length can be changed
[!T!; N] # content and type can be changed
[!T!; !N] # Can perform all sorts of mutable operations
```

Of course, you don't have to memorize and use all of them.
For variable array types, just add `!` to the part you want to be variable, and practically `[T; N]`, `[T!; N]`, `[T; !N]`, ` [T!; !N]` can cover most cases.

These array types are syntactic sugar, the actual types are:

```python
# actually 4 types
[T; N] = Array(T, N)
[T; !N] = Array!(T, !N)
[!T; N] = ArrayWithMutType!(!T, N)
[!T; !N] = ArrayWithMutTypeAndLength!(!T, !N)
[T!; !N] = Array!(T!, !N)
[!T!; N] = ArrayWithMutType!(!T!, N)
[!T!; !N] = ArrayWithMutTypeAndLength!(!T!, !N)
```

This is what it means to be able to change the type.

```python
a = [1, 2, 3].into [!Nat; 3]
a.map!(_ -> "a")
a: [!Str; 3]
```

The same is true for other collection types.

```python
# tuple type
## immutable types
(T, U) # No change in number of elements, contents cannot be changed
## mutable types
(T!, U) # constant number of elements, first element can be changed
(T, U)! # No change in number of elements, content can be replaced
...
```

```python
# Set type
## immutable types
{T; N} # number of immutable elements, contents cannot be changed
## mutable types
{T!; N} # number of immutable elements, content can be changed (one by one)
{T; N}! # Number of variable elements, content cannot be changed
{T!; N}! # Number of variable elements, content can be changed
...
```

```python
# Dictionary type
## immutable types
{K: V} # immutable length, contents cannot be changed
## mutable types
{K: V!} # constant length, values ​​can be changed (one by one)
{K: V}! # Variable length, content cannot be changed, but can be added or deleted by adding or removing elements, content type can also be changed
...
```

```python
# Record type
## immutable types
{x = Int; y = Str} # content cannot be changed
## mutable types
{x = Int!; y = Str} # can change the value of x
{x = Int; y = Str}! # replace any instance of {x = Int; y = Str}
...
```

A type `(...)` that simply becomes `T! = (...)!` when `T = (...)` is called a simple structured type. A simple structured type can also be said (semantically) to be a type that has no internal structure.
Arrays, tuples, sets, dictionaries, and record types are all non-simple structured types, but Int and Refinement types are.

```python
# Refinement type
## Enums
{1, 2, 3} # one of 1, 2, 3, cannot be changed
{1, 2, 3}! # 1, 2, 3, you can change
## interval type
1..12 # 1 to 12, cannot be changed
1..12! # Any of 1-12, you can change
## Refinement type (general type)
{I: Int | I % 2 == 0} # even type, immutable
{I: Int! | I % 2 == 0} # even type, can be changed
{I: Int | I % 2 == 0}! # Exactly the same type as above, but the above notation is preferred
```

From the above explanation, mutable types include not only those that are themselves mutable, but also those whose internal types are mutable.
Types such as `{x: Int!}` and `[Int!; 3]` are internal mutable types where the object inside is mutable and the instance itself is not mutable.

For a type `K!(T, U)` that has internal structure and has a `!` on the type constructor itself, `*self` can change the whole object. Local changes are also possible.
However, it is desirable to keep the change authority as local as possible, so if only `T` can be changed, it is better to use `K(T!, U)`.
And for the type `T!` which has no internal structure, this instance is simply a box of `T` which can be swapped. A method cannot change the type.

---

<span id="1" style="font-size:x-small"><sup>1</sup> It is intentional that `T!` and `T` types have no special linguistic relationship. It's a design. If there is a relationship, for example, if the `T`/`T!` type exists in the namespace, it will not be possible to introduce the `T!`/`T` type from another module. Also, the mutable type is not uniquely defined for the immutable type. Given the definition `T = (U, V)`, the possible variable subtypes of `T!` are `(U!, V)` and `(U, V!)`. [↩](#f1)</span>

<p align='center'>
    <a href='./17_type_casting.md'>Previous</a> | <a href='./19_bound.md'>Next</a>
</p>

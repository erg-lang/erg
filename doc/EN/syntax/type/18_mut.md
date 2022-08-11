# Mutable Type

> __Warning__: The information in this section is out of date and contains some errors.

In Erg, by default, all types are immutable, i.e., their internal state cannot be updated.
However, variable types can of course be defined. Variable types are declared with `!`.

```erg
Person! = Class({name = Str; age = Nat!})
Person!
    greet! ref! self = print! "Hello, my name is {self::name}. I am {self::age}."
    inc_age! ref! self = self::name.update! old -> old + 1
```

To be precise, types whose base type is a variable type or a composite type that contains a variable type must end the type name with `!`! Types without `!` may exist in the same namespace and be treated as separate types.
In the example above, the `.age` attribute is variable and the `.name` attribute is immutable. If any one attribute is variable, the whole is a variable type.

A variable type can define procedural methods to rewrite instances, but just because it has procedural methods does not necessarily make it a variable type. For example, the array type `[T; N]` implements a `sample!` method that randomly selects elements, but this of course does not make destructive changes to the array.

Destructive manipulation of variable type objects is done primarily via the `.update!` method. The `.update!` method is a higher-order procedure that applies the function `f` to `self` to update it.

```erg
i = !1
i.update! old -> old + 1
assert i == 2
```

The `.set!` method simply discards the old content and replaces it with the new value. `.set! x = .update! _ -> x`.

```erg
i = !1
i.set! 2
assert i == 2
```

The `.freeze_map` method invariantizes the value and performs the operation.

```erg
a = [1, 2, 3].into [Nat; !3]
x = a.freeze_map a: [Nat; 3] -> a.iter().map(i -> i + 1).filter(i -> i % 2 == 0).collect(Array)
```

In polymorphic invariant types, the type parameter `T` of a type is implicitly assumed to be an invariant type.

```erg
# ImmutType < Type
K T: ImmutType = Class ...
K! T: Type = Class ...
```

In the standard library, mutable types `T!` are often based on immutable types `T`. However, `T!` and `T` types have no special linguistic relationship, and may not be constructed as such [<sup id="f1">1</sup>](#1) .

Note that there are several types of object mutability.
Below we will review the immutable/mutable semantics of the built-in collection types.

``` erg
# array type
## immutable types
[T; N] # Cannot perform mutable operations
## mutable types
[T!; N] # can change contents one by one
[T; !N] # variable length, content is immutable but can be modified by adding/deleting elements
[!T; N] # The content is an immutable object, but it can be replaced with an object whose type has been changed.
[!T; !N] # type and length can be changed.
[T!; !N] # contents and length can be changed.
[T!; N] # contents and type can be changed.
[T!; !N] # any variable operation can be performed
```

Of course, it is not necessary to memorize and use all of these.
For variable array types, all you have to do is append `!`, and for practical use, `[T; N]`, `[T!; N]`, `[T; !N]`, and `[T!; !N]` can cover most cases.

These array types are sugar-coated constructions, and the actual types are as follows.

```erg
# There are actually five types.
[T; N] = ArrayWithLength(T, N)
[T!; N] = ArrayWithLength!(T!, N)
[T; !N] = ArrayWithMutLength!(T, !N)
[!T; N] = ArrayWithMutType!
[!T; !N] = ArrayWithMutTypeAndLength!
[T!; !N] = ArrayWithMutLength!
[!T!; N] = ArrayWithMutType!
[!T!; !N] = ArrayWithMutTypeAndLength!(!T!, !N)
```

Note that type changeable means this.

```erg
a = [1, 2, 3].into [!Nat; 3].
a.map!(_ -> "a")
a: [!Str; 3].
```

The same is true for other collection types.

```erg
## tuple types
## immutable types
(T, U) ## immutable types: number of elements, contents unchangeable
## mutable types
(T!, U) # number of elements immutable, first element can be changed
(T, U)! # number of elements invariant, contents can be replaced
...
```

```erg
## set types
## immutable types
{T; N} ## number of immutable elements, contents cannot be changed
## mutable types
{T!; N} ## number of immutable types, contents can be changed (one by one)
{T; N}!       ## variable number of elements, contents are unchangeable, but can be changed by adding or deleting elements, and the type inside can be changed.
{T!; N}!      # variable number of elements, contents can be changed
...
```

```erg
# dictionary types.
## immutable types
{K: V} ## invariant length, contents cannot be changed
## mutable types
{K: V!} ## invariant length, can change values (one by one)
{K: V}!         ## variable length, cannot change content, but can add/delete elements, can change type inside
...
```

```erg
# Record type.
## immutable types
{x = Int; y = Str} ## cannot change content
## mutable types
{x = Int!; y = Str} # The value of x can be changed
{x = Int; y = Str}!          ## can replace any instance of {x = Int; y = Str}!
...
```

If `T = (...)` and simply `T! = (...)!`, The type `(...)` is called a simple structural type. A simple structural type can also be said to be a type that (semantically) has no internal structure.
Arrays, tuples, sets, dictionaries, and record types are not all simple structure types, but Int and sieve types are.

```erg
## refinement type
## enumerated types
{1, 2, 3} # one of 1, 2, 3, cannot change
{1, 2, 3}!   ## any of 1, 2, or 3, can be changed
## Interval type
1..12 # one of 1~12, cannot be changed
1..12! # any of 1~12, can be changed
## Sieve type (general form)
{I: Int | I % 2 == 0} # even type, cannot change
{I: Int! | I % 2 == 0} # even type, can be changed
{I: Int | I % 2 == 0}! # exactly the same type as above, but the notation above is recommended
```

From the above explanation, a variable type is not only one that is variable itself, but also one whose internal type is variable.
Types such as `{x: Int!}` and `[Int!; 3]` are internal variable types in which the inner object is variable and not the instance itself.

Types that have an internal structure and the type constructor itself is `! In the case of the type `K!(T, U)` with `! Local modifications are also possible.
However, since it is desirable to keep modification privileges as local as possible, it is better to use `K(T!, U)` when only `T` can be changed.
And in the case of type `T!`, which has no internal structure, this instance is simply a replaceable `T` box. The type cannot be changed by methods.

---

<span id="1" style="font-size:x-small"><sup>1</sup> It is intentional that `T!` and `T` types have no special linguistic relationship. It is a design. If there is a relationship, for example, if the `T`/`T!` type exists in the namespace, it will not be possible to introduce the `T!`/`T` type from another module. Also, the mutable type is not uniquely defined for the immutable type. Given the definition `T = (U, V)`, the possible variable subtypes of `T!` are `(U!, V)` and `(U, V!)`. [↩](#f1)</span>

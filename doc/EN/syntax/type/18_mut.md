# Mutable Type

> __Warning__: The information in this section is old and contains some errors.

By default all types in Erg are immutable, i.e. their internal state cannot be updated.
But you can of course also define mutable types. Variable types are declared with `!`.

```python
Person! = Class({name = Str; age = Nat!})
Person!.
    greet! ref! self = print! "Hello, my name is \{self::name}. I am \{self::age}."
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
x = a.freeze_map a: [Nat; 3] -> a.iter().map(i -> i + 1).filter(i -> i % 2 == 0).collect(List)
```

In a polymorphic immutable type the type argument `T` of the type is implicitly assumed to be immutable.

```python
# ImmutType < Type
KT: ImmutType = Class ...
K!T: Type = Class ...
```

In the standard library, mutable `(...)!` types are often based on immutable `(...)` types. However, `T!` and `T` types have no special linguistic relationship, and may not be constructed as such [<sup id="f1">1</sup>](#1) .

From the above explanation, mutable types include not only those that are themselves mutable, but also those whose internal types are mutable.
Types such as `{x: Int!}` and `[Int!; 3]` are internal mutable types where the object inside is mutable and the instance itself is not mutable.

## Cell! T

Mutable types are already available for `Int` and arrays, but how can we create mutable types for general immutable types? For example, in the case of `{x = Int; y = Int}`, corresponding mutable type is `{x = Int!; y = Int!}`, etc. But how did `Int!` made from `Int`?

Erg provides `Cell!` type for such cases.
This type is like a box for storing immutable types. This corresponds to what is called a reference (ref) in ML and other languages.

```python
IntOrStr = Inr or Str
IntOrStr! = Cell! IntOrStr
x = IntOrStr!.new 1
assert x is! 1 # `Int or Str` cannot compare with `Int` directly, so use `is!` (this compares object IDs) instead of `==`.
x.set! "a"
assert x is! "a"
```

An important property is that `Cell! T` is a subtype of `T`. Therefore, an object of type `Cell! T` can use all the methods of type `T`.

```python
# definition of `Int!`
Int! = Cell! Int
...

i = !1
assert i == 1 # `i` is casted to `Int`
```

---

<span id="1" style="font-size:x-small"><sup>1</sup> It is intentional that `T!` and `T` types have no special linguistic relationship. It's a design. If there is a relationship, for example, if the `T`/`T!` type exists in the namespace, it will not be possible to introduce the `T!`/`T` type from another module. Also, the mutable type is not uniquely defined for the immutable type. Given the definition `T = (U, V)`, the possible variable subtypes of `T!` are `(U!, V)` and `(U, V!)`. [↩](#f1)</span>

<p align='center'>
    <a href='./17_type_casting.md'>Previous</a> | <a href='./19_bound.md'>Next</a>
</p>

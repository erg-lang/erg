# Resolving patch methods

`Nat` is zero or more `Int`, a subtype of `Int`.
`Nat` does not exist in the Python class hierarchy. I wonder how Erg solves this patch method?

```python
1.times do:
    log "hello world"
```

`.times` is a `NatImpl` patch method.
Since `1` is an instance of `Int`, it is first searched by tracing the MRO (Method Resolution Order) of `Int`.
Erg has `Int`, `Object` in the MRO of `Int`. It comes from Python (`int.__mro__ == [int, object]` in Python).
The `.times` method does not exist in either of them. Now let's explore that subtype.

~

Integers should obviously have reals, complexes, and even whole numbers in their supertypes, but that fact does not appear in the Python-compatible layer.
However, `1 in Complex` and `1 in Num` are actually `True` in Erg.
As for `Complex`, even though it is a class that does not have an inheritance relationship with `Int`, it is judged to be compatible as a type. What the hell is going on?

~

An object has an infinite number of types to which it belongs.
But we really only have to think about types with methods, i.e. types with names.

The Erg compiler has a hashmap of patch types with all provided methods and their implementations.
This table is updated each time a new type is defined.

```python
provided_method_table = {
    ...
    "foo": [Foo],
    ...
    ".times": [Nat, Foo],
    ...
}
```

Types that have a `.times` method are `Nat`, `Foo`. From among these, find one that matches the `{1}` type.
There are two types of conformity determination. They are refinement-type judgment and record-type judgment. This is done from the refinement type determination.

## Refinement type determination

Check if the candidate type is compatible with the type `{1}` of `1`. The refinement types compatible with `{1}` are `{0, 1}`, `0..9`, and so on.
Finite element algebraic types such as `0..1 or 3..4`, `-1..2 and 0..3` are normalized to refinement types when declared as base types (i.e. ` {0, 1, 3, 4}`, `{0, 1, 2}`).
In this case, `Nat` is `0.._ == {I: Int | I >= 0}`, so `{1}` is compatible with `Nat`.

## Determine record type

Check if the candidate type is compatible with `Int`, a class of 1.
Others that are patches of `Int` and that `Int` has all the required attributes are also compatible.

~

So `Nat` fit. However, if `Foo` also matches, it is determined by the containment relationship between `Nat` and `Foo`.
That is, subtype methods are selected.
If there is no containment relationship between the two, a compile error will occur (this is a safety measure against executing a method against the programmer's intention).
To eliminate the error, you need to specify the patch explicitly.

```python
o.method(x) -> P.method(o, x)
```

## method resolution for universal patches

Define a patch like this:

```python
FnType T: Type = Patch T -> T
FnType.type = T
```

Code like the following is possible under the `FnType` patch. I wonder how this will be resolved.

```python
assert (Int -> Int).type == Int
```

First, `FnType(T)` is registered in `provided_method_table` in the following format.

```python
provided_method_table = {
    ...
    "type": [FnType(T)],
    ...
}
```

`FnType(T)` is checked for matching types. In this case, `FnType(T)` patch type is `Type -> Type`.
This matches `Int -> Int`. If it fits, do monomorphization and replace (take a diff of `T -> T` and `Int -> Int`, `{T => Int}`).

```python
assert FnType(Int).type == Int
```

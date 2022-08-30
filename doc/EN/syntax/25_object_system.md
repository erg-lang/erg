# Object

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/25_object_system.md%26commit_hash%3D6c6afe84d1dc05ee7566b46c12d39b8c49a3acfb)
](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/25_object_system.md&commit_hash=6c6afe84d1dc05ee7566b46c12d39b8c49a3acfb)

All data that can be assigned to a variable. The `Object` class has the following attributes.

* `. __repr__`: returns a (non-rich) string representation of the object.
* `. __sizeof__`: returns the size of the object (including heap allocation).
* `. __dir__`: return a list of attributes of the object.
* `. __hash__`: return the hash value of the object.
* `. __getattribute__`: retrieve and return an object's attributes * `.
* `.clone`: create and return a clone of an object (an independent entity in memory).
* `.copy`: return a copy of an object (identical in memory).

## Record

An object created by a record literal (`{attr = value; ...}`).
This object can be a `.clone` or a `. __sizeof__` and other basic methods.

```erg
obj = {.x = 1}
assert obj.x == 1

obj2 = {. .x; .y = 2}
assert obj2.x == 1 and obj2.y == 2
```

## Attribute

An object associated with an object. In particular, a subroutine attribute that takes itself (`self`) as its implicit first argument is called a method.

```erg
# Note that private_attr does not have `. Note that there is no `.
record = {.public_attr = j; private_attr = 2; .method = self -> self.i + 1}
record.public_attr == 2
record.private_attr # AttributeError: private_attr is private
assert record.method() == 3
```

## Element

An object belonging to a specific type (e.g. `1` is an element of type `Int`). All objects are at least `{=}` type.
In the case of an element of a class, it is sometimes called an instance.

## Subroutine

An object that is an instance of a function or procedure (including methods). The class representing a subroutine is `Subroutine`.
More generally, `.__call__` is called a `Callable`.

## Callable

Object that implements `.__call__`. Superclass of `Subroutine`.

## Type

An object that defines required attributes and makes objects common.
There are two main types: polymorphic type and monomorphic type. Typical monomorphic types are `Int`, `Str`, etc. Polymorphic types include `Option Int`, `[Int; 3]` and so on.
In addition, types that define methods to change the state of an object are called mutable types, and require variable attributes marked with `!` (e.g., dynamic arrays: `[T; !_]`).

## Function

Subroutines that have read permission for external variables (excluding static variables) but do not have read/write permission for external variables. In other words, it has no external side effects.
Erg functions are defined differently than Python because they do not allow side effects.

## Procedure

It has read and `self` permissions for external variables, read/write permissions for static variables, and is allowed to use all subroutines. It can have external side effects.

## Method

A subroutine that implicitly takes `self` as the first argument. It is a different type than a simple function/procedure.

## Entity

Objects that are not subroutines and types.
Monomorphic entities (`1`, `"a"`, etc.) are also called value objects, and polymorphic entities (`[1, 2, 3], {"a": 1}`) are also called container objects.

<p align='center'>
    <a href='./24_module.md'>Previous</a> | <a href='./26_pattern_matching.md'>Next</a>
</p>

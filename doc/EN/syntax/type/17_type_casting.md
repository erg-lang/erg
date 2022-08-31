# Cast

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/17_type_casting.md%26commit_hash%3D2f89a30335024a46ec0b3f6acc6d5a4b8238b7b0)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/17_type_casting.md&commit_hash=2f89a30335024a46ec0b3f6acc6d5a4b8238b7b0)

## Upcasting

Because Python is a language that uses duck typing, there is no concept of casting. There is no need to upcast, and there is essentially no downcasting.
However, Erg is statically typed, so there are times when casting must be done.
A simple example is `1 + 2.0`: the `+`(Int, Ratio), or Int(<: Add(Ratio, Ratio)) operation is not defined in the Erg language specification. This is because `Int <: Ratio`, so 1 is upcast to 1.0, an instance of Ratio.

~~The Erg extended bytecode adds type information to BINARY_ADD, in which case the type information is Ratio-Ratio. In this case, the BINARY_ADD instruction does the casting of Int, so no special instruction specifying the cast is inserted. So, for example, even if you override a method in a child class, if you specify the parent as the type, type coercion is performed and the method is executed in the parent's method (name modification is performed at compile time to refer to the parent's method). The compiler only performs type coercion validation and name modification. The runtime does not cast objects (currently. Cast instructions may be implemented for execution optimization). ~~

```erg
@Inheritable
Parent = Class()
Parent.
    greet!() = print! "Hello from Parent"

Child = Inherit Parent
Child.
    # Override requires Override decorator
    @Override
    greet!() = print! "Hello from Child"

greet! p: Parent = p.greet!()

parent = Parent.new()
child = Child.new()

parent # "Hello from Parent" greet!
child # "Hello from Parent"
```

This behavior does not create an incompatibility with Python. In the first place, Python does not specify the type of a variable, so that all variables are typed as type variables, so to speak. Since type variables choose the smallest type they can fit, the same behavior as in Python is achieved if you do not specify a type in Erg.

```erg
@Inheritable
Parent = Class()
Parent.
    greet!() = print! "Hello from Parent"

Child = Inherit Parent
Child.
    greet!() = print! "Hello from Child" Child.

greet! some = some.greet!()

parent = Parent.new()
child = Child.new()

parent # "Hello from Parent" greet!
child # "Hello from Child"
```

You can also use `.from` and `.into`, which are automatically implemented for types that are inherited from each other.

```erg
assert 1 == 1.0
assert Ratio.from(1) == 1.0
assert 1.into<Ratio>() == 1.0
```

## Downcasting

Since downcasting is generally unsafe and the conversion method is non-trivial, we instead implement ``TryFrom.try_from``.

```erg
IntTryFromFloat = Patch Int
IntTryFromFloat.
    try_from r: Float =
        if r.ceil() == r:
            then: r.ceil()
            else: Error "conversion failed".
```

# Cast

## Upcasting

Because Python is a language that uses duck typing, there is no concept of casting. There is no need to upcast, and there is essentially no downcasting.
However, Erg is statically typed, so there are times when casting must be done.
A simple example is `1 + 2.0`: the `+`(Int, Ratio), or Int(<: Add(Ratio, Ratio)) operation is not defined in the Erg language specification. This is because `Int <: Ratio`, so 1 is upcast to 1.0, an instance of Ratio.

~~The Erg extended bytecode adds type information to BINARY_ADD, in which case the type information is Ratio-Ratio. In this case, the BINARY_ADD instruction does the casting of Int, so no special instruction specifying the cast is inserted. So, for example, even if you override a method in a child class, if you specify the parent as the type, type coercion is performed and the method is executed in the parent's method (name modification is performed at compile time to refer to the parent's method). The compiler only performs type coercion validation and name modification. The runtime does not cast objects (currently. Cast instructions may be implemented for execution optimization). ~~

```python
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

```python
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

```python
assert 1 == 1.0
assert Ratio.from(1) == 1.0
assert 1.into<Ratio>() == 1.0
```

## Forced upcasting

In many cases, upcasting of objects is automatic, depending on the function or operator that is called.
However, there are cases when you want to force upcasting. In that case, you can use `as`.

```python,compile_fail
n = 1
n.times! do: print!
    print! "Hello"

i = n as Int
i.times! do: # ERR
    "Hello"

s = n as Str # ERR
```

You cannot cast to unrelated types or subtypes with ``as``.

## Forced casting

You can use `typing.cast` to force casting. This can convert the target to any type.
In Python, `typing.cast` does nothing at runtime, but in Erg the conversion will be performed by the constructor if object's type is built-in[<sup id="f1">1</sup>](#1).
For non-built-in types, the safety is not guaranteed at all.

```python
typing = pyimport "typing"

C = Class { .x = Int }

s = typing.cast Str, 1

assert s == "1"
print! s + "a" # 1a

c = typing.cast C, 1
print! c.x # AttributeError: 'int' object has no attribute 'x'
```

## Downcasting

Since downcasting is generally unsafe and the conversion method is non-trivial, we instead implement ``TryFrom.try_from``.

```python
IntTryFromFloat = Patch Int
IntTryFromFloat.
    try_from r: Float =
        if r.ceil() == r:
            then: r.ceil()
            else: Error "conversion failed".
```

---

<span id="1" style="font-size:x-small"><sup>1</sup> This conversion is a byproduct of the current implementation and will be removed in the future. [↩](#f1) </span>

<p align='center'>
    <a href='./16_subtyping.md'>Previous</a> | <a href='./18_mut.md'>Next</a>
</p>

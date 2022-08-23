# Phantom class

Phantom types are marker traits that exist only to provide annotations to the compiler.
As a usage of phantom types, let's look at the structure of a list.

```erg
Nil = Class()
List T, 0 = Inherit Nil
List T, N: Nat = Class {head = T; rest = List(T, N-1)}
```

This code results in an error.

```erg
3 | List T, 0 = Inherit Nil
                        ^^^
TypeConstructionError: since Nil does not have a parameter T, it is not possible to construct List(T, 0) with Nil
hint: use 'Phantom' trait to consume T
```

This error is a complaint that `T` cannot be type inferred when `List(_, 0).new Nil.new()` is used.
In such a case, whatever the `T` type is, it must be consumed on the right-hand side. A type of size zero, such as a tuple of length zero, is convenient because it has no runtime overhead.

```erg
Nil T = Class((T; 0))
List T, 0 = Inherit Nil T
List T, N: Nat = Class {head = T; rest = List(T, N-1)}
```

This code passes compilation. But it's a little tricky to understand the intent, and it can't be used except when the type argument is a type.

In such a case, a phantom type is just what you need. A phantom type is a generalized type of size 0.

```erg
Nil T = Class(Impl: Phantom T)
List T, 0 = Inherit Nil T
List T, N: Nat = Class {head = T; rest = List(T, N-1)}

nil = Nil(Int).new()
assert nil.__size__ == 0
```

`Phantom` holds the type `T`. But in fact the size of the `Phantom T` type is 0 and does not hold an object of type `T`.

Also, `Phantom` can consume arbitrary type arguments in addition to its type. In the following example, `Phantom` holds a type argument called `State`, which is a subtype object of `Str`.
Again, `State` is a fake type variable that does not appear in the object's entity.

```erg
VM! State: {"stopped", "running"}! = Class(... State)
VM!("stopped").
    start ref! self("stopped" ~> "running") =
        self.do_something!()
        self::set_phantom!("running"))
```

The `state` is updated via the `update_phantom!` or `set_phantom!` methods.
This is the method provided by the standard patch for `Phantom!` (the variable version of `Phantom`), and its usage is the same as the variable `update!` and `set!`.

# dependent type

Dependent types are a feature that can be said to be the biggest feature of Erg.
A dependent type is a type that takes a value as an argument. Ordinary polymorphic types can take only types as arguments, but dependent types relax that restriction.

Dependent types are equivalent to `[T; N]` (`Array(T, N)`).
This type is determined not only by the content type `T` but also by the number of contents `N`. `N` contains an object of type `Nat`.

``` erg
a1 = [1, 2, 3]
assert a1 in [Nat; 3]
a2 = [4, 5, 6, 7]
assert a1 in [Nat; 4]
assert a1 + a2 in [Nat; 7]
```

If the type object passed in the function argument is related to the return type, write:

``` erg
narray: |N: Nat| {N} -> [{N}; N]
narray(N: Nat): [N; N] = [N; N]
assert array(3) == [3, 3, 3]
```

When defining a dependent type, all type arguments must be constants.

Dependent types themselves exist in existing languages, but Erg has the feature of defining procedural methods on dependent types.

``` erg
x=1
f x =
    print! f::x, module::x

# The Phantom type has an attribute called Phantom whose value is the same as the type argument
T X: Int = Class Impl := Phantom X
T(X).
    x self = self::Phantom

T(1).x() # 1
```

Type arguments of mutable dependent types can be transitioned by method application.
Transition specification is done with `~>`.

``` erg
# Note that `Id` is an immutable type and cannot be transitioned
VM!(State: {"stopped", "running"}! := _, Id: Nat := _) = Class(..., Impl := Phantom! State)
VM!().
    # Variables that do not change can be omitted by passing `_`.
    start! ref! self("stopped" ~> "running") =
        self.initialize_something!()
        self::set_phantom!("running")

# You can also cut out by type argument (only in the module where it's defined)
VM!.new() = VM!(!"stopped", 1).new()
VM!("running" ~> "running").stop!ref!self =
    self.close_something!()
    self::set_phantom!("stopped")

vm = VM!.new()
vm.start!()
vm.stop!()
vm.stop!() # TypeError: VM!(!"stopped", 1) doesn't have .stop!()
# hint: VM!(!"running", 1) has .stop!()
```

You can also embed or inherit existing types to create dependent types.

``` erg
MyArray(T, N) = Inherit[T; N]

# The type of self: Self(T, N) changes in conjunction with .array
MyStruct!(T, N: Nat!) = Class {.array: [T; !N]}
```
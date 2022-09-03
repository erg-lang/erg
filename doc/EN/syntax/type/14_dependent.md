# Dependent Types

Dependent types are one of the most important features of Erg.
Dependent types are types that take values as arguments. Normal polymorphic types can take only types as arguments, but dependent types loosen that restriction.

Dependent types, such as `[T; N]`(`Array(T, N)`), are equivalent.
This type depends not only on the type `T` of the contents, but also on the number `N` of the contents. `N` contains objects of type `Nat`.

```erg
a1 = [1, 2, 3].
assert a1 in [Nat; 3].
a2 = [4, 5, 6, 7]
assert a1 in [Nat; 4].
assert a1 + a2 in [Nat; 7].
```

If the type object passed as a function argument is related to a return type, write the following

```erg
narray: |N: Nat| {N} -> [{N}; N]
narray(N: Nat): [N; N] = [N; N]
assert narray(3) == [3, 3, 3].
```

When defining a dependent type, all type arguments must be constants.

Dependent types already exist in some languages, but Erg has the unique feature of allowing you to define procedural methods on dependent types.

```erg
x = 1
f x =
    print! f::x, module::x

# Phantom types have an attribute called Phantom that has the same value as the type argument
T X: Int = Class Impl := Phantom X
T(X).
    x self = self::Phantom

T(1).x() # 1
```

Type arguments of variable-dependent types can be transitioned by applying methods.
Transitions are specified with `~>`.

```erg
# Note that `Id` is an immutable type and cannot be transitioned.
VM!(State: {"stopped", "running"}! := _, Id: Nat := _) = Class(... State).
VM!().
    # Variables that do not change can be omitted by passing `_`.
    start! ref! self("stopped" ~> "running") =
        self.initialize_something!
        self::set_phantom!("running")

# You can also cut out each type argument (only within the defined module)
VM!.new() = VM!(!" stopped", 1).new()
VM!("running" ~> "running").stop! ref! self =
    self.close_something!()
    self::set_phantom!("stopped"))

vm = VM!.new()
vm.start!()
vm.stop!()
vm.stop!() # TypeError: VM! stopped", 1) doesn't have .stop!
# TypeError: VM! running", 1) has .stop!
```

You can also create dependent types by incorporating or inheriting from existing types.

```erg
MyArray(T, N) = Inherit [T; N].

# type of self: Self(T, N) in conjunction with .array!
MyStruct!(T, N: Nat!) = Class {.array: [T; !N]}
```

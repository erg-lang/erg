# variation

Erg can subtype polymorphic types, but there are some caveats.

First, consider the inclusion relation of ordinary polymorphic types. In general, there is a container `K` and a type `A, B` to which it assigns, and when `A < B`, `K A < K B`.
For example, `Option Int < Option Object`. Therefore, 方法 defined in `Option Object` can also be used in `Option Int`.

Consider the typical polymorphic type `Array!(T)`.
Note that this time it's not `Array!(T, N)` because we don't care about the number of elements.
Now, the `Array!(T)` type has 方法 called `.push!` and `.pop!`, which mean adding and removing elements, respectively. Here is the type:

Array.push!: Self(T).(T) => NoneType
Array.pop!: Self(T).() => T

As can be intuitively understood,

* `Array!(Object).push!(s)` is OK when `s: Str` (just upcast `Str` to `Object`)
* When `o: Object`, `Array!(Str).push!(o)` is NG
* `Array!(Object).pop!().into(Str)` is NG
* `Array!(Str).pop!().into(Object)` is OK

is. In terms of the type system, this is

* (Self(Object).(Object) => NoneType) < (Self(Str).(Str) => NoneType)
* (Self(Str).() => Str) < (Self(Object).() => Object)

means

The former may seem strange. Even though `Str < Object`, the inclusion relation is reversed in the function that takes it as an argument.
In type theory, such a relation (the type relation of `.push!`) is called contravariant, and vice versa, the type relation of `.pop!` is called covariant.
In other words, function types are contravariant with respect to their argument types and covariant with respect to their return types.
It sounds complicated, but as we saw earlier, it's a reasonable rule if you apply it to an actual example.
If you still don't quite get it, consider the following.

One of Erg's design principles is "large input types, small output types". This is precisely the case for function mutability.
Looking at the rules above, the larger the input type, the smaller the overall type.
This is because general-purpose functions are clearly rarer than special-purpose functions.
And the smaller the output type, the smaller the whole.

As a result, the above policy is equivalent to saying "minimize the type of the function".

## Immutability

Erg has another modification. It is non-variance.
This is a modification that built-in types such as `SharedCell! T!` have. This means that for two types `T!, U!` where `T! != U!`, casts between `SharedCell! T!` and `SharedCell! means that
This is because `SharedCell! T!` is a shared reference. See [shared references](shared.md) for details.

## Mutated generic type

A universal type variable can specify its upper and lower bounds.

``` erg
|A <: T| K(A)
|B :> T| K(B)
```

In the type variable list, the __variant specification__ of the type variable is performed. In the above variant specification, the type variable `A` is declared to be any subclass of type `T` and the type variable `B` is declared to be any superclass of type `T`.
In this case, `T` is also called the upper type for `A` and the lower type for `B`.

Mutation specifications can also overlap.

``` erg
# U<A<T
{... | A<: T; A :> U}
```

Here is an example of code that uses a variable specification.

``` erg
show|S <: Show| s: S = log s

Nil T = Class(Impl = Phantom T)
Cons T = Class(Nil T or List T)
List T = Class {head = T; rest = Cons T}
List(T).
    push|U <: T|(self, x: U): List T = Self. new {head = x; rest = self}
    upcast(self, U :> T): List U = self
```

## Change specification

The `List T` example is tricky, so let's go into a little more detail.
To understand the code above, you need to know about polymorphic type degeneration. Variance is discussed in detail in [this section](./variance.md), but for now we need three facts:

* Ordinary polymorphic types, such as `List T`, are covariant with `T` (`List U > List T` when `U > T`)
* The function `T -> U` is contravariant with respect to the argument type `T` (`(S -> U) < (T -> U)` when `S > T`)
* Function `T -> U` is covariant with return type `U` (`(T -> U) > (T -> S)` when `U > S`)

For example, `List Int` can be upcast to `List Object` and `Obj -> Obj` can be upcast to `Int -> Obj`.

Now let's consider what happens if we omit the variable specification of the method.

``` erg
...
List T = Class {head = T; rest = Cons T}
List(T).
    # List T can be pushed U if T > U
    push|U|(self, x: U): List T = Self. new {head = x; rest = self}
    # List T can be List U if T < U
    upcast(self, U): List U = self
```

Even in this case, the Erg compiler does a good job of inferring the upper and lower types of `U`.
Note, however, that the Erg compiler doesn't understand the semantics of 方法. The compiler simply infers and derives type relationships mechanically according to how variables and type variables are used.

As written in the comments, the type `U` put in the `head` of `List T` is a subclass of `T` (`T: Int`, such as `Nat`). That is, it is inferred as `U <: T`. This constraint changes the argument type of `.push{U}` upcast `(List(T), U) -> List(T) to (List(T), T) -> List(T)`( e.g. disallow `List(Int).push{Object}`). Note, however, that the `U <: T` constraint does not alter the type containment of the function. The fact that `(List(Int), Object) -> List(Int) to (List(Int), Int) -> List(Int)` does not change, just in `.push` method It means that the cast cannot be performed.
Similarly, a cast from `List T` to `List U` is possible subject to the constraint `U :> T`, so the variation specification is inferred. This constraint changes the return type of `.upcast(U)` to upcast `List(T) -> List(T) to List(T) -> List(T)` (e.g. `List(Object) .upcast(Int)`) is prohibited.

Now let's see what happens if we allow this upcast.
Let's invert the denaturation designation.

``` erg
...
List T = Class {head = T; rest = Cons T}
List(T).
    push|U :> T|(self, x: U): List T = Self. new {head = x; rest = self}
    upcast(self, U :> T): List U = self
# TypeWarning: `U` in the `.push` cannot take anything other than `U == T`. Replace `U` with `T`.
# TypeWarning: `U` in the `.upcast` cannot take anything other than `U == T`. Replace `U` with `T`.
```

Both the constraint `U <: T` and the modification specification `U :> T` are satisfied only when `U == T`. So this designation doesn't make much sense.
Only "upcasts such that `U == T`" = "upcasts that do not change where `U`" are actually allowed.

## Appendix: Modification of user-defined types

Mutations of user-defined types are immutable by default. However, you can also specify mutability with the `Inputs/Outputs` marker trait.
If you specify `Inputs(T)`, the type is contravariant with respect to `T`.
If you specify `Outputs(T)`, the type is covariant with respect to `T`.

``` erg
K T = Class(...)
assert not K(Str) <= K(Object)
assert not K(Str) >= K(Object)

InputStream T = Class ..., Impl := Inputs(T)
# A stream that accepts Objects can also be considered to accept Strs
assert InputStream(Str) > InputStream(Object)

OutputStream T = Class ..., Impl := Outputs(T)
# A stream that outputs a Str can also be considered to output an Object
assert OutputStream(Str) < OutputStream(Object)
```
# Type Variable, quantified type

A type variable is a variable used, for example, to specify the type of subroutine arguments, and its type is arbitrary (not monomorphic).
First, as motivation for introducing type variables, consider the `id` function, which returns input as is.

```python
id x: Int = x
```

The `id` function that returns the input as is is defined for the type `Int`, but this function can obviously be defined for any type.
Let's use `Object` for the largest class.

```python
id x: Object = x

i = id 1
s = id "foo"
b = id True
```

Sure, it now accepts arbitrary types, but there is one problem: the return type is expanded to `Object`. The return type is expanded to `Object`.
I would like to see the return type `Int` if the input is of type `Int`, and `Str` if it is of type `Str`.

```python
print! id 1 # <Object object>
id(1) + 1 # TypeError: cannot add `Object` and `Int
```

To ensure that the type of the input is the same as the type of the return value, use a __type variable__.
Type variables are declared in `||`(type variable list).

```python
id|T: Type| x: T = x
assert id(1) == 1
assert id("foo") == "foo"
assert id(True) == True
```

This is called the __universal quantification (universalization)__ of the function. There are minor differences, but it corresponds to the function called generics in other languages. A universalized function is called a __polymorphic function__.
Defining a polymorphic function is like defining a function of the same form for all types (Erg prohibits overloading, so the code below cannot really be written).

```python
id|T: Type| x: T = x
# pseudo code
id x: Int = x
id x: Str = x
id x: Bool = x
id x: Ratio = x
id x: NoneType = x
...
```

Also, the type variable `T` can be inferred to be of type `Type` since it is used in the type specification. So `|T: Type|` can simply be abbreviated to `|T|`.
You can also omit `|T, N| foo: [T; N]` if it can be inferred to be other than a type object (`T: Type, N: Nat`).

You can also provide constraints if the type is too large for an arbitrary type.
Constraints also have advantages, for example, a subtype specification allows certain 方法 to be used.

```python
# T <: Add
# => T is a subclass of Add
# => can do addition
add|T <: Add| l: T, r: T = l + r
```

In this example, `T` is required to be a subclass of type `Add`, and the actual types of `l` and `r` to be assigned must be the same.
In this case, `T` is satisfied by `Int`, `Ratio`, etc. So, the addition of `Int` and `Str`, for example, is not defined and is therefore rejected.

You can also type it like this.

```python
f|
    Y, Z: Type
    X <: Add Y, O1
    O1 <: Add Z, O2
    O2 <: Add X, _
| x: X, y: Y, z: Z =
    x + y + z + x
```

If the annotation list is long, you may want to pre-declare it.

```python
f: |Y, Z: Type, X <: Add(Y, O1), O1 <: Add(Z, O2), O2 <: Add(X, O3)| (X, Y, Z) -> O3
f|X, Y, Z| x: X, y: Y, z: Z =
    x + y + z + x
```

Unlike many languages with generics, all declared type variables must be used either in the temporary argument list (the `x: X, y: Y, z: Z` part) or in the arguments of other type variables.
This is a requirement from Erg's language design that all type variables are inferrable from real arguments.
So information that cannot be inferred, such as the return type, is passed from real arguments; Erg allows types to be passed from real arguments.

```python
Iterator T = Trait {
    # Passing return types from arguments.
    # .collect: |K: Type -> Type| Self(T). ({K}) -> K(T)
    .collect(self(T), K: Type -> Type): K(T) = ...
    ...
}

it = [1, 2, 3].iter().map i -> i + 1
it.collect(Array) # [2, 3, 4].
```

Type variables can only be declared during `||`. However, once declared, they can be used anywhere until they exit scope.

```python
f|X|(x: X): () =
    y: X = x.clone()
    log X.__name__
    log X

f 1
# Int
# <class Int>
```

You can also explicitly monophasize at the time of use as follows

```python
f: Int -> Int = id|Int|
```

In that case, the specified type takes precedence over the type of the actual argument (failure to match will result in a type error that the type of the actual argument is wrong).
That is, if the actual object passed can be converted to the specified type, it will be converted; otherwise, a compile error will result.

```python
assert id(1) == 1
assert id|Int|(1) in Int
assert id|Ratio|(1) in Ratio
# You can also use keyword arguments
assert id|T: Int|(1) == 1
id|Int|("str") # TypeError: id|Int| is type `Int -> Int` but got Str
```

When this syntax is batting against comprehensions, you need to enclose it in `()`.

```python
# {id|Int| x | x <- 1..10} would be interpreted as {id | ...} will be interpreted as.
{(id|Int| x) | x <- 1..10}
```

A type variable cannot be declared with the same name as a type that already exists. This is because all type variables are constants.

```python
I: Type
# ↓ invalid type variable, already exists
f|I: Type| ... = ...
```

## Type arguments in method definitions

Type arguments on the left-hand side are treated as bound variables by default.

```python
K(T: Type, N: Nat) = ...
K(T, N).
    foo(x) = ...
```

Using another type variable name will result in a warning.

```python
K(T: Type, N: Nat) = ...
K(U, M). # Warning: K's type variable names are 'T' and 'N'
    foo(x) = ...
```

Constants are the same in all namespaces since their definition, so of course they cannot be used for type variable names.

```python
N = 1
K(N: Nat) = ... # NameError: N is already defined

L(M: Nat) = ...
# 定义ined only if M == N == 1
L(N).
    foo(self, x) = ...
# 定义ined for any M: Nat
L(M).
    .bar(self, x) = ...
```

You cannot have multiple definitions for each type argument, but you can define 方法 with the same name because there is no relationship between dependent types that are not assigned type arguments (non-primitive-kind) and dependent types that are assigned (primitive-kind).

```python
K(I: Int) = ...
K.
    # K is not a true type (atomic Kind), so we cannot define a method
    # This is not a method (more like a static method)
    foo(x) = ...
K(0).
    foo(self, x): Nat = ...
```

## All symmetric types

The `id` function defined in the previous section is a function that can be of any type. So what is the type of the `id` function itself?

```python
print! classof(id) # |T: Type| T -> T
```

We get a type `|T: Type| T -> T`. This is called a __closed universal quantified type/universal type__, which is `['a. ...]'` in ML, and `forall t. ...` in Haskell. Why the adjective "closed" is used is discussed below.

There is a restriction on the closed universal quantified type: only subroutine types can be made universal quantified, i.e., only subroutine types can be placed in the left clause. But this is sufficient, since subroutines are the most basic control structure in Erg, so when we say "I want to handle arbitrary X," i.e., I want a subroutine that can handle arbitrary X. So, the quantified type has the same meaning as the polymorphic function type. From now on, this type is basically called polymorphic function type.

Like anonymous functions, polymorphic types have arbitrary type variable names, but they all have the same value.

```python
assert (|T: Type| T -> T) == (|U: Type| U -> U)
```

The equality is satisfied when there is an alpha equivalence, as in the lambda calculus. Since there are some restrictions on operations on types, equivalence determination is always possible (if we don't consider the stoppage property).

## Subtyping of Polymorphic Function Types

A polymorphic function type can be any function type. This means that there is a subtype relationship with any function type. Let's look at this relationship in detail.

A type in which the type variable is defined on the left-hand side and used on the right-hand side, such as `OpenFn T: Type = T -> T`, is called an __open universal type__.
In contrast, a type in which type variables are defined and used on the right-hand side, such as `ClosedFn = |T: Type| T -> T`, is called a __closed universal type__.

An open universal type is a supertype of all isomorphic "true" types. In contrast, a closed universal type is a subtype of all isomorphic true types.

```python
(|T: Type| T -> T) < (Int -> Int) < (T -> T)
```

You may remember that closed ones are smaller/open ones are larger.
But why is this so? For a better understanding, let's consider an instance of each.

```python
# id: |T: Type| T -> T
id|T|(x: T): T = x

# iid: Int -> Int
iid(x: Int): Int = x

# return arbitrary function as is
id_arbitrary_fn|T|(f1: T -> T): (T -> T) = f
# id_arbitrary_fn(id) == id
# id_arbitrary_fn(iid) == iid

# return the poly correlation number as it is
id_poly_fn(f2: (|T| T -> T)): (|T| T -> T) = f
# id_poly_fn(id) == id
id_poly_fn(iid) # TypeError

# Return Int type function as is
id_int_fn(f3: Int -> Int): (Int -> Int) = f
# id_int_fn(id) == id|Int|
# id_int_fn(iid) == iid
```

Since `id`, which is of type `|T: Type| T -> T`, can be assigned to a parameter `f3` of type `Int -> Int`, we may consider `(|T| T -> T) < (Int -> Int)`.
Conversely, `iid`, which is of type `Int -> Int`, cannot be assigned to parameter `f2` of type `(|T| T -> T)`, but it can be assigned to parameter `f1` of type `T -> T`, so `(Int -> Int) < (T -> T)`.
Therefore, it is indeed `(|T| T -> T) < (Int -> Int) < (T -> T)`.

## Quantified Types and Dependent Types

What is the relationship between dependent types and quantified types (polymorphic function types) and what is the difference between them?
We can say that a dependent type is a type that takes arguments, and an quantified type is a type that gives arbitrariness to the arguments.

The important point is that there are no type arguments in the closed, polymorphic type itself. For example, the polymorphic function type `|T| T -> T` is a type that takes a polymorphic function __only__, and its definition is closed. You cannot define 方法, etc. using its type argument `T`.

In Erg, the type itself is also a value, so types that take arguments, such as function types, will probably be dependent types. In other words, polymorphic function types are both a quantified type and a dependent type.

```python
PolyFn = Patch(|T| T -> T)
PolyFn.
    type self = T # NameError: cannot find 'T'
DepFn T = Patch(T -> T)
DepFn.
    type self =
        log "by DepFn"
        T

assert (Int -> Int).type() == Int # by DepFn
assert DepFn(Int).type() == Int # by DepFn
```

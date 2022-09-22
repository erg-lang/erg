# type inference algorithm

> __Warning__: This section is being edited and may contain some errors.

The notation used below is shown.

```python
Free type variables (type, unbound): ?T, ?U, ...
Free-type variables (values, unbound): ?a, ?b, ...
type environment (Γ): { x: T, ... }
Type assignment rule (S): { ?T --> T, ... }
Type argument evaluation environment (E): { e -> e', ... }
```

Let's take the following code as an example.

```python
v = ![]
v.push! 1
print! v
```

Erg's type inference largely uses the Hindley-Milner type inference algorithm (although various extensions have been made). Specifically, type inference is performed by the following procedure. Terminology will be explained later.

1. Infer the type of the rvalue (search)
2. instantiate the resulting type
3. If it is a call, perform type substitution (substitute)
4. Resolve traits that have already been monomorphized
5. Evaluate/reduce (eval) if there is a type variable value
6. Remove linked type variables (deref)
7. Propagate changes for mutable dependent methods
8. If there is an lvalue and it is Callable, generalize the argument type (generalize)
9. If there is an lvalue, generalize the (return value) type (generalize)
10. If it is an assignment, register the type information in the symbol table (`Context`) (update)

The specific operations are as follows.

line 1. Def{sig: v, block: ![]}
    get block type:
        get UnaryOp type:
            getArray type: `['T; 0]`
            instantiate: `[?T; 0]`
            (substitute, eval are omitted)
    update: `Γ: {v: [?T; 0]!}`
    expr returns `NoneType`: OK

line 2. CallMethod {obj: v, name: push!, args: [1]}
    get obj type: `Array!(?T, 0)`
        search: `Γ Array!(?T, 0).push!({1})`
        get: `= Array!('T ~> 'T, 'N ~> 'N+1).push!('T) => NoneType`
        instantiate: `Array!(?T, ?N).push!(?T) => NoneType`
        substitute(`S: {?T --> Nat, ?N --> 0}`): `Array!(Nat ~> Nat, 0 ~> 0+1).push!(Nat) => NoneType`
        eval: `Array!(Nat, 0 ~> 1).push!({1}) => NoneType`
        update: `Γ: {v: [Nat; 1]!}`
    expr returns `NoneType`: OK

line 3. Call {obj: print!, args: [v]}
    get args type: `[[Nat; 1]!]`
    get obj type:
        search: `Γ print!([Nat; 1]!)`
        get: `= print!(...Object) => NoneType`
    expr returns `NoneType`: OK

## Implementation of type variables

Type variables were originally expressed as follows in `Type` of [ty.rs]. It's now implemented in a different way, but it's essentially the same idea, so I'll consider this implementation in a more naive way.
`RcCell<T>` is a wrapper type for `Rc<RefCell<T>>`.

```rust
pub enum Type {
    ...
    Var(RcCell<Option<Type>>), // a reference to the type of other expression, see docs/compiler/inference.md
    ...
}
```

A type variable can be implemented by keeping the entity type in an external dictionary, and the type variable itself only has its keys. However, it is said that the implementation using `RcCell` is generally more efficient (verification required, [source](https://mobile.twitter.com/bd_gfngfn/status/1296719625086877696?s=21) ).

A type variable is first initialized as `Type::Var(RcCell::new(None))`.
This type variable is rewritten as the code is analyzed, and the type is determined.
If the content remains None until the end, it will be a type variable that cannot be determined to a concrete type (on the spot). For example, the type of `x` with `id x = x`.
I'll call a type variable in this state an __Unbound type variable__ (I don't know the exact terminology). On the other hand, we call a variable that has some concrete type assigned to it a __Linked type variable__.

Both are of the kind free type variables (the term is apparently named after "free variables"). These are type variables that the compiler uses for inference. It has such a special name because it is different from a type variable whose type is specified by the programmer, such as `'T` in `id: 'T -> 'T`.

Unbound type variables are expressed as `?T`, `?U`. In the context of type theory, α and β are often used, but this one is used to simplify input.
Note that this is a notation adopted for general discussion purposes and is not actually implemented using string identifiers.

An unbound type variable `Type::Var` is replaced with a `Type::MonoQuantVar` when entering a type environment. This is called a __quantified type variable__. This is akin to the programmer-specified type variables, such as ``T``. The content is just a string, and there is no facility to link to a concrete type like a free-type variable.

The operation of replacing unbound type variables with quantified type variables is called __generalization__ (or generalization). If you leave it as an unbound type variable, the type will be fixed with a single call (for example, after calling `id True`, the return type of `id 1` will be `Bool`), so It has to be generalized.
In this way a generalized definition containing quantified type variables is registered in the type environment.

## generalizations, type schemes, reifications

Let's denote the operation of generalizing an unbound type variable `?T` as `gen`. Let the resulting generalized type variable be `|T: Type| T`.
In type theory, quantified types, such as the polycorrelation type `α->α`, are distinguished by prefixing them with `∀α.` (symbols like ∀ are called (generic) quantifiers. ).
Such a representation (e.g. `∀α.α->α`) is called a type scheme. A type scheme in Erg is denoted as `|T: Type| T -> T`.
Type schemes are not usually considered first-class types. Configuring the type system that way can prevent type inference from working. However, in Erg, it can be regarded as a first-class type under certain conditions. See [rank2 type](../syntax/type/advanced/rank2type.md) for details.

Now, when using the obtained type scheme (e.g. `'T -> 'T (id's type scheme)`) in type inference where it is used (e.g. `id 1`, `id True`), generalize must be released. This inverse transformation is called __instantiation__. We will call the operation `inst`.

```python
gen ?T = 'T
inst 'T = ?T (?T ∉ Γ)
```

Importantly, both operations replace all occurrences of the type variable. For example, if you instantiate `'T -> 'T`, you get `?T -> ?T`.
A replacement dict is required for instantiation, but for generalization, just link `?T` with `'T` to replace it.

After that, give the type of the argument to get the target type. This operation is called type substitution, and will be denoted by `subst`.
In addition, the operation that obtains the return type if the expression is a call is denoted as `subst_call_ret`. The first argument is a list of argument types, the second argument is the type to assign to.

The type substitution rule `{?T --> X}` means to rewrite `?T` and `X` to be of the same type. This operation is called __Unification__. `X` can also be a type variable.
A detailed unification algorithm is described in [separate section]. We will denote the unify operation as `unify`.

```python
unify(?T, Int) == Ok(()) # ?T == (Int)

# S is the type assignment rule, T is the applicable type
subst(S: {?T --> X}, T: ?T -> ?T) == X -> X
# Type assignment rules are {?T --> X, ?U --> T}
subst_call_ret([X, Y], (?T, ?U) -> ?U) == Y
```

## semi-unification

A variant of unification is called semi-unification (__Semi-unification__). This is the operation that updates the type variable constraints to satisfy the subtype relation.
In some cases, type variables may or may not be unifying, hence the term "semi" unification.

Semi-unification occurs, for example, during argument assignment.
because the type of the actual argument must be a subtype of the type of the formal argument.
If the argument type is a type variable, we need to update the subtype relation to satisfy it.

```python
# If the formal parameter type is T
f(x: T): T = ...

a: U
# must be U <: T, otherwise type error
f(a)
```

## Generalization

Generalization is not a simple task. When multiple scopes are involved, "level management" of type variables becomes necessary.
In order to see the necessity of level management, we first confirm that type inference without level management causes problems.
Infer the type of the following anonymous function.

```python
x ->
    y = x
    y
```

First, Erg allocates type variables as follows:
The type of y is also unknown, but is left unassigned for now.

```python
x(: ?T) ->
    y = x
    y
```

The first thing to determine is the type of the rvalue x. An rvalue is a "use", so we reify it.
But the type `?T` of x is already instantiated because it is a free variable. Yo`?T` becomes the type of the rvalue.

```python
x(: ?T) ->
    y = x (: inst ?T)
    y
```

Generalize when registering as the type of lvalue y. However, as we will see later, this generalization is imperfect and produces erroneous results.

```python
x(: ?T) ->
    y(:gen?T) = x(:?T)
    y
```

```python
x(: ?T) ->
    y(: 'T) = x
    y
```

The type of y is now a quantified type variable `'T`. In the next line, `y` is used immediately. Concrete.

```python
x: ?T ->
    y(: 'T) = x
    y(: inst 'T)
```

Note that instantiation must create a (free) type variable that is different from any (free) type variables that already exist (generalization is similar). Such type variables are called fresh type variables.

```python
x: ?T ->
    y = x
    y(: ?U)
```

And look at the type of the resulting whole expression. `?T -> ?U`.
But obviously this expression should be `?T -> ?T`, so we know there is a problem with the reasoning.
This happened because we didn't "level manage" the type variables.

So we introduce the level of type variables with the following notation. Levels are expressed as natural numbers.

```python
# normal type variable
?T<1>, ?T<2>, ...
# type variable with subtype constraint
?T<1>(<:U) or ?T(<:U)<1>, ...
```

Let's try again.

```python
x ->
    y = x
    y
```

First, assign a leveled type variable as follows: The toplevel level is 1. As the scope gets deeper, the level increases.
Function arguments belong to an inner scope, so they are one level higher than the function itself.

```python
# level 1
x (: ?T<2>) ->
    # level 2
    y = x
    y
```

First, instantiate the rvalue `x`. Same as before, nothing changed.

```python
x (: ?T<2>) ->
    y = x (: inst ?T<2>)
    y
```

Here is the key. This is a generalization when assigning to the type of lvalue `y`.
Earlier, the results were strange here, so we will change the generalization algorithm.
If the level of the type variable is less than or equal to the level of the current scope, generalization leaves it unchanged.

```python
gen ?T<n> = if n <= current_level, then= ?T<n>, else= 'T
```

```python
x (: ?T<2>) ->
    # current_level = 2
    y(: gen ?T<2>) = x(: ?T<2>)
    y
```

That is, the lvalue `y` has type `?T<2>`.

```python
x (: ?T<2>) ->
    # ↓ not generalized
    y(: ?T<2>) = x
    y
```

The type of y is now an unbound type variable `?T<2>`. Concrete with the following lines: but the type of `y` is not generalized, so nothing happens.

```python
x (: ?T<2>) ->
    y(: ?T<2>) = x
    y (: inst ?T<2>)
```

```python
x (: ?T<2>) ->
    y = x
    y (: ?T<2>)
```

We successfully got the correct type `?T<2> -> ?T<2>`.

Let's see another example. This is the more general case, with function/operator application and forward references.

```python
fx, y = id(x) + y
id x = x

f10,1
```

Let's go through it line by line.

During the inference of `f`, the later defined function constant `id` is referenced.
In such a case, insert a hypothetical declaration of `id` before `f` and assign a free-type variable to it.
Note that the level of the type variable at this time is `current_level`. This is to avoid generalization within other functions.

```python
id: ?T<1> -> ?U<1>
f x (: ?V<2>), y (: ?W<2>) =
    id(x) (: subst_call_ret([inst ?V<2>], inst ?T<1> -> ?U<1>)) + y
```

Unification between type variables replaces higher-level type variables with lower-level type variables.
It doesn't matter which one if the level is the same.

Semiunification between type variables is a little different.
Type variables at different levels must not impose type constraints on each other.

```python
# BAD
f x (: ?V<2>), y (: ?W<2>) =
    # ?V<2>(<: ?T<1>)
    # ?T<1>(:> ?V<2>)
    id(x) (: ?U<1>) + y (: ?W<2>)
```

This makes it impossible to determine where to instantiate the type variable.
For Type type variables, normal unification is performed instead of semi-unification.
In other words, unify to the lower level.

```python
# OK
f x (: ?V<2>), y (: ?W<2>) =
    # ?V<2> --> ?T<1>
    id(x) (: ?U<1>) + y (: ?W<2>)
```

```python
f x (: ?T<1>), y (: ?W<2>) =
    (id(x) + x): subst_call_ret([inst ?U<1>, inst ?W<2>], inst |'L <: Add('R)| ('L, 'R) -> 'L .AddO)
```

```python
f x (: ?T<1>), y (: ?W<2>) =
    (id(x) + x): subst_call_ret([inst ?U<1>, inst ?W<2>], (?L(<: Add(?R<2>))<2>, ?R<2 >) -> ?L<2>.AddO)
```

```python
id: ?T<1> -> ?U<1>
f x (: ?T<1>), y (: ?W<2>) =
    # ?U<1>(<: Add(?W<2>)) # Inherit the constraints of ?L
    # ?L<2> --> ?U<1>
    # ?R<2> --> ?W<2> (not ?R(:> ?W), ?W(<: ?R))
    (id(x) + x) (: ?U<1>.AddO)
```

```python
# current_level = 1
f(x, y) (: gen ?T<1>, gen ?W<2> -> gen ?U<1>.AddO) =
    id(x) + x
```

```python
id: ?T<1> -> ?U<1>
f(x, y) (: |'W: Type| (?T<1>, 'W) -> gen ?U<1>(<: Add(?W<2>)).AddO) =
    id(x) + x
```

```python
f(x, y) (: |'W: Type| (?T<1>, 'W) -> ?U<1>(<: Add(?W<2>)).AddO) =
    id(x) + x
```

When defining, raise the level so that it can be generalized.

```python
# ?T<1 -> 2>
# ?U<1 -> 2>
id x (: ?T<2>) -> ?U<2> = x (: inst ?T<2>)
```

If the return type has already been assigned, unify with the resulting type (`?U<2> --> ?T<2>`).

```python
# ?U<2> --> ?T<2>
f(x, y) (: |'W: Type| (?T<2>, 'W) -> ?T<2>(<: Add(?W<2>)).AddO) =
    id(x) + x
# current_level = 1
id(x) (: gen ?T<2> -> gen ?T<2>) = x (: ?T<2>)
```

If the type variable has been instantiated into a simple Type variable,
The type variable that depends on it will also be a Type type variable.
Generalized type variables are independent for each function.

```python
f(x, y) (: |'W: Type, 'T <: Add('W)| ('T, 'W) -> 'T.AddO) =
    id(x) + x
id(x) (: |'T: Type| 'T -> gen 'T) = x
```

```python
f x, y (: |'W: Type, 'T <: Add('W)| ('T, 'W) -> 'T.AddO) =
    id(x) + y
id(x) (: 'T -> 'T) = x

f(10, 1) (: subst_call_ret([inst {10}, inst {1}], inst |'W: Type, 'T <: Add('W)| ('T, 'W) -> 'T .AddO)
```

```python
f(10, 1) (: subst_call_ret([inst {10}, inst {1}], (?T<1>(<: Add(?W<1>)), ?W<1>) -> ? T<1>.AddO))
```

Type variables are bounded to the smallest type that has an implementation.

```python
# ?T(:> {10} <: Add(?W<1>))<1>
# ?W(:> {1})<1>
# ?W(:> {1})<1> <: ?T<1> (:> {10}, <: Add(?W(:> {1})<1>))
# serialize
# {1} <: ?W<1> or {10} <: ?T<1> <: Add({1}) <: Add(?W<1>)
# The minimal implementation trait for Add(?W)(:> ?V) is Add(Nat) == Nat, since Add is covariant with respect to the first argument
# {10} <: ?W<1> or {1} <: ?T<1> <: Add(?W<1>) <: Add(Nat) == Nat
# ?T(:> ?W(:> {10}) or {1}, <: Nat).AddO == Nat # If there is only one candidate, finalize the evaluation
f(10, 1) (: (?W(:> {10}, <: Nat), ?W(:> {1})) -> Nat)
# This is the end of the program, so remove the type variable
f(10, 1) (: ({10}, {1}) -> Nat)
```

The resulting type for the entire program is:

```python
f|W: Type, T <: Add(W)|(x: T, y: W): T.AddO = id(x) + y
id|T: Type|(x: T): T = x

f(10, 1): Nat
```

I've also reprinted the original, unexplicitly typed program.

```python
fx, y = id(x) + y
id x = x

f(10, 1)
```
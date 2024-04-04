# Attributes Resolution

Resolving attributes means, for example, given the expression `x.y`, determining the type of this entire expression. Therefore, the type of `x` must be determined, but the type of `x` may not be uniquely determined. In such cases, the type of `x.y` may still be determined, or it may fail. This is the problem of attribute resolution that this section deals with.

As a simple case, consider the expression `1.real` (`x == 1, y == real`). The type of `1` is `{1}`. `{1}` is a subtype of `Nat`, `Int` and `Obj`. Trace these types in turn to find the definition of `real`. In this case, it is found in `Int` (`Int.real: Int`). Thus the type of `x` is cast to `Int` and the type of `1.real` is `Int`.

Thus, if the type of `x` is uniquely determinable, inference proceeds in the order `x` -> `y`.
However, when the type of `x` cannot be uniquely determined, the type of `x` can be narrowed down from `y`.

For example:

```erg
consts c = c.co_consts
```

``co_consts`` is an attribute of type `Code`. This example was chosen just because a name that does not cover others. It is not the essence of what this function means.
Since the type of ``c`` is not specified, You may think inference is not possible, but it is (when the only type with ``co_consts`` in the namespace is `Code`).

Erg assigns a type variable when no variable type is specified.

```erg
consts: ?1
c: ?2
```

The type inferrer tries to determine the definition of `co_consts` from the type `?2`, but fails because `?2` is a type variable with no condition.
In such a case, [`get_attr_type_by_name`](https://github.com/erg-lang/erg/blob/b8a87c0591e5603c1afcfc54c073ab2101ff2857/crates/erg_compiler/context/inquire.rs#L2884) is called.
This method attempts to identify the type `?2` from the name `co_consts`, as opposed to before.
It succeeds only if the only type with `co_consts` in the namespace is `Code` (or all other types are supertype of Code).
Erg closes function type checking within a module, so even if a type with `co_consts` is defined outside the module, passing an instance of it to the `consts` function will result in an error (to make this possible, you must use `Structural`, described below). This constraint allows the `consts` function to infer.

When a class attribute is defined, the type inferrer keeps track of the "attribute" and "defining class, attribute type" pairs.
In the case of ``co_consts``, this pair is `{co_consts: {Code, List(Obj, _)}}`.

```erg
method_to_classes: {co_consts: [{Code, List(Obj, _)}], real: [{Int, Int}], times!: [{Nat, (self: Nat, proc!: () => NoneType) => NoneType}], ...}
```

Note that the value of the key-value pair is an array. Only if this array is of length 1, or has the smallest type element, the key is uniquely determined (otherwise a type error will occur).

Once the key is identified, the definition type is back-propagated to the type of ``?2``.

```erg
?2(<: Code).co_consts: List(Obj, _)
```

Finally, the type of `consts` is `Code -> List(Obj, _)`.

```erg
consts(c: Code): List(Obj, _) = c.co_consts
```

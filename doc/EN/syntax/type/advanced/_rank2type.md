# rank-2 polymorphism

> __Warning__: This document is out of date and contains errors in general.

Erg allows you to define functions that accept various types such as `id|T|(x: T): T = x`, ie polycorrelations.
So, can we define a function that accepts polycorrelations?
For example, a function like this (note that this definition is erroneous):

```python
# I want tuple_map(i -> i * 2, (1, "a")) == (2, "aa")
tuple_map|T|(f: T -> T, tup: (Int, Str)): (Int, Str) = (f(tup.0), f(tup.1))
```

Note that `1` and `"a"` have different types, so the anonymous function is not monomorphic once. Needs to be single-phased twice.
Such a function cannot be defined within the scope of the types we have discussed so far. This is because type variables have no notion of scope.
Let's leave the types for a moment and see the concept of scope at the value level.

```python
arr = [1, 2, 3]
arr.map i -> i + 1
```

`arr` and `i` in the code above are variables in different scopes. Therefore, each life span is different (`i` is shorter).

The types so far have the same lifetime for all type variables. In other words, `T`, `X`, and `Y` must be determined at the same time and remain unchanged thereafter.
Conversely, if we can think of `T` as a type variable in the "inner scope", we can compose a `tuple_map` function. __Rank 2 type__ was prepared for that purpose.

```python
# tuple_map: ((|T: Type| T -> T), (Int, Str)) -> (Int, Str)
tuple_map f: (|T: Type| T -> T), tup: (Int, Str) = (f(tup.0), f(tup.1))
assert tuple_map(i -> i * 2, (1, "a")) == (2, "aa")
```

A type of the form `{(type) | (list of type variables)}` is called a universal type (see [Universal type](./../quantified.md) for details).
The `id` function we have seen so far is a typical universal function = polycorrelation function.

```python
id x = x
id: |T: Type| T -> T
```

A universal type has special rules for association with the function type constructor `->`, and the semantics of the type are completely different depending on the way of association.

Think about this in terms of simple one-argument functions.

```python
f1: (T -> T) -> Int | T # a function that takes any function and returns an Int
f2: (|T: Type| T -> T) -> Int # Function that receives polycorrelation and returns Int
f3: Int -> (|T: Type| T -> T) # A function that takes an Int and returns a closed universal function
f4: |T: Type|(Int -> (T -> T)) # Same as above (preferred)
```

It seems strange that `f1` and `f2` are different, while `f3` and `f4` are the same. Let's actually construct a function of such a type.

```python
# id: |T: Type| T -> T
id x = x
# same type as `f1`
take_univq_f_and_return_i(_: (|T: Type| T -> T), i: Int): Int = i
# same type as `f2`
take_arbit_f_and_return_i|T: Type|(_: T -> T, i: Int): Int = i
# same type as `f3`
take_i_and_return_univq_f(_: Int): (|T: Type| T -> T) = id
# same type as `f4`
take_i_and_return_arbit_f|T: Type|(_: Int): (T -> T) = id
```

After applying it, you will notice the difference.

```python
_ = take_univq_f_and_return_i(x -> x, 1) # OK
_ = take_univq_f_and_return_i(x: Int -> x, 1) #NG
_ = take_univq_f_and_return_i(x: Str -> x, 1) # NG
_ = take_arbit_f_and_return_i(x -> x, 1) # OK
_ = take_arbit_f_and_return_i(x: Int -> x, 1) # OK
_ = take_arbit_f_anf_return_i(x: Str -> x, 1) # OK

f: |T| T -> T = take_i_and_return_univq_f(1)
g: |T| T -> T = take_i_and_return_arbit_f(1)
assert f == g
f2: Int -> Int = take_i_and_return_univq_f|Int|(1)
g2: Int -> Int = take_i_and_return_arbit_f|Int|(1)
assert f2 == g2
```

An open polycorrelation function type is specifically called an __arbitrary function type__. Arbitrary function types have an infinite number of possibilities: `Int -> Int`, `Str -> Str`, `Bool -> Bool`, `|T: Type| T -> T`, ... be.
On the other hand, there is only one closed (returning an object of the same type as the argument) polymorphic type `|T: Type| T -> T`. Such types are specifically called __polymorphic function types__.
In other words, `f1` can be passed `x: Int -> x+1`, `x: Bool -> not x`, `x -> x`, etc. = `f1` is a polycorrelated number Yes, but you can only pass `x -> x` etc. to `f2` = `f2` is not __a polycorrelation__.
But the types of functions like `f2` are clearly different from normal types, and we need new concepts to handle them well. That is the "rank" of the type.

Regarding the definition of rank, types that are not quantified, such as `Int`, `Str`, `Bool`, `T`, `Int -> Int`, `Option Int`, etc., are treated as "rank 0".

```python
# K is a polynomial kind such as Option
R0 = (Int or Str or Bool or ...) or (R0 -> R0) or K(R0)
```

Next, types with first-order universal quantification, such as `|T| T -> T`, or types that include them in the return value type are "rank 1".
In addition, types with second-order universal quantification (types that have rank 1 types as arguments such as `(|T| T -> T) -> Int`) or types that include them in the return type are called "rank 2 ".
The above is repeated to define the "Rank N" type. Also, the rank-N types include all types with a rank of N or less. Therefore, a type with mixed ranks has the same rank as the highest among them.

```python
R1 = (|...| R0) or (R0 -> R1) or K(R1) or R0
R2 = (|...| R1) or (R1 -> R2) or K(R2) or R1
...
Rn = (|...| Rn-1) or (Rn-1 -> Rn) or K(Rn) or Rn-1
```

Let's look at some examples.

```python
    (|T: Type| T -> T) -> (|U: Type| U -> U)
=> R1 -> R1
=> R1 -> R2
=> R2

Option(|T: Type| T -> T)
=> Option(R1)
=> K(R1)
=> R1
```

By definition, `tuple_map` is a rank-2 type.

```python
tuple_map:
    ((|T: Type| T -> T), (Int, Str)) -> (Int, Str)
=> (R1, R0) -> R0
=> R1 -> R2
=> R2
```

Erg can handle types up to rank 2 (because rank N types include all types with rank N or less, to be exact, all Erg types are rank 2 types). Attempting to construct a function of more types is an error.
For example, all functions that handle polycorrelations as they are require specification of other argument types. Also, such a function is not configurable.

```python
# this is a rank-3 type function
# |X, Y: Type|((|T: Type| T -> T), (X, Y)) -> (X, Y)
generic_tuple_map|X, Y: Type| f: (|T: Type| T -> T), tup: (X, Y) = (f(tup.0), f(tup.1))
```

It is known that types with rank 3 or higher are theoretically undecidable by type inference. However, most practical needs can be covered by the rank 2 type.
# Variable

Variables are a type of algebra; algebra in Erg - sometimes simply referred to as variable if there is no confusion - refers to the feature to name objects and make them referable from elsewhere in the code.

A variable is defined as follows.
The `n` part is called the variable name (or identifier), `=` is the assignment operator, and the `1` part is the assigned value.

```erg
n = 1
```

The `n` defined in this way can thereafter be used as a variable to denote the integer object `1`. This system is called assignment (or binding).
We have just said that `1` is an object. We will discuss what an object is later, but for now we will assume that it is something that can be assigned to, i.e., on the right side of the assignment operator (`=`, etc.).

If you want to specify the "type" of a variable, do the following. The type is roughly the set to which an object belongs, as will be explained later.
Here we specify that `n` is a natural number (`Nat`) type.

```erg
n: Nat = 1
```

Note that, unlike other languages, multiple assignments are not allowed.

```erg
# NG
l1 = l2 = [1, 2, 3] # SyntaxError: multiple assignment not allowed
# OK
l1 = [1, 2, 3]
l2 = l1.clone()
```

It is also not possible to reassign to a variable. The syntax that can be used instead, to hold mutable states, are described later.

```erg
i = 1
i = i + 1 # AssignError: cannot assign twice
```

You can define a variable with the same name in the inner scope, but you are only covering it over, not destructively rewriting its value. If you go back to the outer scope, the value will return as well.
Note that this is a different behavior than the Python "statement" scope.
This kind of functionality is generally referred to as shadowing. However, unlike shadowing in other languages, you cannot shadow in the same scope.

```erg
x = 0
# x = 1 # AssignError: cannot assign twice
if x.is_zero(), do:
    x = 1 # different from outer x with same name
    assert x == 1
assert x == 0
```

The following may seem possible at first glance, but it is still not possible. This is a design decision, not a technical constraint.

```erg
x = 0
if x.is_zero(), do:
    x = x + 1 # NameError: cannot define variables refer to variables with the same name
    assert x == 1
assert x == 0
```

## Constants

Constants are also a type of algebra. If you start an identifier with a capital letter, it is treated as a constant. They are called constants because once defined, they do not change.
The `N` part is called the constant name (or identifier). Otherwise, it is the same as a variable.

```erg
N = 0
if True, do:
    N = 1 # AssignError: constants cannot be shadowed
    pass()
```

Constants are immutable beyond the defined scope. They cannot be shadowed. Because of this property, constants can be used in pattern matching. Pattern matching is explained later.

For example, constants are used for mathematical constants, information about external resources, and other immutable values.

It is common practice to use all-caps (style in which all letters are capitalized) for identifiers of objects other than [types](./type/01_type_system.md).

```erg
PI = 3.141592653589793
URL = "https://example.com"
CHOICES = ["a", "b", "c"]
```

```erg
PI = 3.141592653589793
match! x:
    PI => print! "π"
    other => print! "other"
```

The above code prints `π` when `x` is `3.141592653589793`. If `x` is changed to any other number, it prints `other`.

Some objects cannot be bound as constants. Mutable objects, for example. Mutable objects are objects whose states can be changed, as described in detail later.
This is because of the rule that only constant expressions can be assigned to constants. Constant expressions are also discussed later.

```erg
X = 1 # OK
X = !1 # TypeError: cannot define Int! object as a constant
```

## Delete an Variable

You can delete an variable by using the `Del` function. All other variables that depend on the variable (that is, that refer directly to the value of the variable) are also removed.

```erg
x = 1
y = 2
Z = 3
f a = x + a

assert f(2) == 3
Del x
Del y, Z

f(2) # NameError: f is not defined (deleted in line 6)
```

Note that `Del` can only delete variables defined in the user-defined module. Built-in constants such as `True` cannot be deleted.

```erg
Del True # TypeError: cannot delete built-in constants
Del print! # TypeError: cannot delete built-in variables
```

## Appendix: Assignment and Equivalence

Note that `x == a` is not necessarily true when `x = a`. An example is `Float.NaN`. This is the formal specification of floating-point numbers as defined by IEEE 754.

```erg
x = Float.NaN
assert x ! = NaN
assert x ! = x
```

There are other objects for which no equivalence relation is defined in the first place.

```erg
f = x -> x**2 + 2x + 1
g = x -> (x + 1)**2
f == g # TypeError: cannot compare function objects

C = Class {i: Int}
D = Class {i: Int}
C == D # TypeError: cannot compare class objects
```

Strictly speaking, `=` does not assign the right-hand side value directly to the left-hand side identifier.
In the case of function and class objects, "modification" such as giving variable name information to the object is performed. However, this is not the case for structural types.

```erg
f x = x
print! f # <function f>
g x = x + 1
print! g # <function g>

C = Class {i: Int}
print! C # <class C>
```

<p align='center'>
    <a href='. /01_literal.md'>Previous</a> | <a href='. /03_declaration.md'>Next</a>
</p>

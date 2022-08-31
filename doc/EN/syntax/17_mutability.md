# Mutability

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/17_mutability.md%26commit_hash%3D21e8145e83fb54ed77e7631deeee8a7e39b028a3)
](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/17_mutability.md&commit_hash=21e8145e83fb54ed77e7631deeee8a7e39b028a3)

As we have already seen, all Erg variables are immutable. However, Erg objects have the concept of mutability.
Take the following code as an example.

```erg
a = [1, 2, 3]
a = a + [4, 5, 6]
print! a # [1, 2, 3, 4, 5, 6]
```

The above code cannot actually be executed by Erg. This is because it is not reassignable.

This code can be executed.

```erg
b = ![1, 2, 3]
b.concat! [4, 5, 6]
print! b # [1, 2, 3, 4, 5, 6]
```

The final result of `a, b` looks the same, but their meanings are very different.
Although `a` is a variable that represents an array of `Nat`, the objects pointed to in the first and second lines are different. The name `a` is the same, but the contents are different.

```erg
a = [1, 2, 3]
print! id! a # 0x000002A798DFE940
_a = a + [4, 5, 6]
print! id! _a # 0x000002A798DFE980
```

The `id!` procedure returns the address in memory where the object resides.

`b` is a `Nat` "dynamic" array. The content of the object changes, but the variables point to the same thing.

```erg
b = ![1, 2, 3]
print! id! b # 0x000002A798DFE220
b.concat! [4, 5, 6]
print! id! b # 0x000002A798DFE220
```

```erg
i = !0
if! True. do!
    do! i.inc!() # or i.add!(1)
    do pass
print! i # 1
```

`!` is a special operator called the __mutation operator__. It makes immutable objects mutable.
The behavior of objects marked with `!` can be customized.

```erg
Point = Class {.x = Int; .y = Int}

# In this case .x is made mutable and .y remains immutable
Point! = Class {.x = Int!; .y = Int}
Point!.
    inc_x! ref!(self) = self.x.update! x -> x + 1

p = Point!.new {.x = !0; .y = 0}
p.inc_x!()
print! p.x # 1
```

## Constant

Unlike variables, constants point to the same thing in all scopes.
Constants are declared with the `=` operator.

```erg
PI = 3.141592653589
match! x:
    PI => print! "this is pi"
```

Constants are identical in all scopes below the global and cannot be overwritten. Therefore, they cannot be redefined by ``=``. This restriction allows it to be used in pattern matching.
The reason why `True` and `False` can be used in pattern matching is because they are constants.
Also, constants always point to immutable objects. Types such as `Str!` cannot be constants.
All built-in types are constants because they should be determined at compile time. Types that are not constants can be generated, but they cannot be used to specify a type and can only be used like a simple record. Conversely, a type is a record whose contents are determined at compile time.

## Variable, Name, Identifier, Symbol

Let's clear up a few terms related to variables in Erg.

A Variable is a mechanism to give an object a Name so that it can be reused (or point to that Name).
Identifier is a grammar element that specifies a variable.
A symbol is a grammatical element, a token, that represents a name.

Only non-symbolic characters are symbols, and symbols are not called symbols, although they can be identifiers as operators.
For example, `x` is an identifier and a symbol. `x.y` is also an identifier, but it is not a symbol. `x` and `y` are symbols.
And even if `x` were not tied to any object, `x` would still be a Symbol and an Identifier, but it would not be called a Variable.
Identifiers of the form `x.y` are called Field Accessors.
Identifiers of the form `x[y]` are called Subscript Accessors.

The difference between a variable and an identifier is that if we are talking about a variable in the sense of Erg's grammatical theory, the two are in effect the same.
In C, types and functions cannot be assigned to variables; int and main are identifiers, but not variables (strictly speaking, they can be assigned, but there are restrictions).
However, in Erg, "everything is an object". Not only functions and types, but even operators can be assigned to variables.

<p align='center'>
    <a href='./16_iterator.md'>Previous</a> | <a href='./18_ownership.md'>Next</a>
</p>

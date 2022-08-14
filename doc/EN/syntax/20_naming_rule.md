# Naming Conventions

If a variable is to be used as a constant expression, it must begin with a capital letter. The second and succeeding letters may be in lowercase.

```erg
i: Option Type = Int
match i:
    t: Type -> log "type"
    None -> log "None"
```

Objects with side-effects must end with `!` must end with `!`. They are procedures, procedural methods, and mutable types.
However, the `Proc` type itself is not a mutable type.

```erg
# Callable == Func or Proc
c: Callable = print!
match c:
    p! -> log "proc" # can omit `: Proc` since it is self-explanatory
    f -> log "func"
```

If you want to expose the attribute to the outside world, define it with `.`. `.` attribute is not prefixed, the attribute is not public. To avoid confusion, they cannot coexist in the same scope.

```erg
o = {x = 1; .x = 2} # SyntaxError: private and public variables with the same name cannot coexist
```

## Literal Identifiers

The above rule can be avoided by enclosing the string in single quotes (''). That is, a procedural object can also be assigned without `!`. In this case, however, even if the value is a constant expression, it is not considered a constant.
Such a string identifier enclosed in single quotes is called a literal identifier.
This is used when calling the API (FFI) of other languages such as Python.

```erg
bar! = pyimport("foo").'bar'
```

Identifiers that are also valid in Erg do not need to be enclosed in ''.

Furthermore, literal identifiers can contain both symbols and spaces, so strings that cannot normally be used as identifiers can be used as identifiers.

```erg
'∂/∂t' y
'test 1: pass x to y'()
```

<p align='center'>
    <a href='./19_visibility.md'>Previous</a> | <a href='./21_lambda.md'>Next</a>
</p>

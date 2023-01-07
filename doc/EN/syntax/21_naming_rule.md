# Naming convention

If you want to use a variable as a constant expression, make sure it starts with a capital letter. Two or more letters may be lowercase.

```python
i: Option Type = Int
match i:
    t: Type -> log "type"
    None -> log "None"
```

Objects with side effects always end with `!`. Procedures and procedural methods, and mutable types.
However, the `Proc` type itself is not mutable.

```python
# Callable == Func or Proc
c: Callable = print!
match c:
    p! -> log "proc" # `: Proc` can be omitted since it is self-explanatory
    f -> log "func"
```

If you want to expose an attribute to the outside world, define it with `.` at the beginning. If you don't put `.` at the beginning, it will be private. To avoid confusion, they cannot coexist within the same scope.

```python,compile_fail
o = {x = 1; .x = 2} # SyntaxError: private and public variables with the same name cannot coexist
```

## Literal Identifiers

The above rule can be circumvented by enclosing the string in single quotes (''). That is, procedural objects can also be assigned without `!`. However, in this case, even if the value is a constant expression, it is not considered a constant.
A character string enclosed in single quotes like this is called a literal identifier.
This is used when calling APIs (FFI) of other languages ​​such as Python.

```python
bar! = pyimport("foo").'bar'
```

Identifiers that are also valid in Erg do not need to be enclosed in ''.

Furthermore, literal identifiers can contain both symbols and spaces, so strings that cannot normally be used as identifiers can be used as identifiers.

```python
'∂/∂t' y
'test 1: pass x to y'()
```

<p align='center'>
    <a href='./20_visibility.md'>Previous</a> | <a href='./22_lambda.md'>Next</a>
</p>
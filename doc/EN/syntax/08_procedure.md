# Procedures

Procedures mean the functions that allow [side-effect](/07_side_effect.md).
Please refer to [Function](/04_function.md) basically usage or definition.
Add `!` to a function name to define it.

```python
proc!(x: Int!, y: Int!) =
    for! 0..x, i =>
        for 0..y, j =>
            print! i, j
```

Procedures are necessary when dealing with mutable objects, but having a mutable object as an argument does not necessarily make it a procedure.
Here is a function takes a mutable object (not procedure).

```python
peek_str s: Str! = log s

make_proc(x!: (Int => Int)): (Int => Int) = y => x! y
p! = make_proc(x => x)
print! p! 1 # 1
```

Also, procedures and functions are related by `proc :> func`.
Therefore, it is possible to define functions in procedure.
However, note that the reverse is not possible.

```python
proc!(x: Int!) = y -> log x, y # OK
func(x: Int) = y => print! x, y # NG
```

<p align='center'>
    <a href='./07_side_effect.md'>Previous</a> | <a href='./09_builtin_procs.md'>Next</a>
</p>

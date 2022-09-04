# Procedures

Procedures are necessary when dealing with mutable objects, but having a mutable object as an argument does not necessarily make it a procedure.
Here is a function takes a mutable object (not procedure).

```erg
peek_str s: Str! = log s
```

<p align='center'>
    <a href='./07_side_effect.md'>Previous</a> | <a href='./09_builtin_procs.md'>Next</a>
</p>

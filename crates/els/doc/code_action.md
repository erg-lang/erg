# Available Code Actions

## `eliminate_unused_vars`

This code action will eliminate unused variables in code.

```erg
foo = 1
for! 0..3, i =>
    print! 1
```

↓

```erg
for! 0..3, _ =>
    print! 1
```

## `change_case`

This code action will change non-snake case variables to snake case.

```erg
fooBar = 1
```

↓

```erg
foo_bar = 1
```

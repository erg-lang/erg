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

## `inline_variables`

This code action will inline variables.

```erg
two = 1 + 1
print! two
print! two * 1
```

↓

```erg
print! 1 + 1
print!((1 + 1) * 1)
```

## `extract_variables`

This code action will extract variables.

```erg
print! |1 + 1| # |...| is selected
```

↓

```erg
new_var = 1 + 1
print! new_var
```

## `extract_functions`

This code action will extract functions.

```erg
if foo, do:
    # |...| is selected
    |for arr, i =>
        ...
    bar()|
```

↓

```erg
new_func() =
    for arr, i =>
        ...
    bar()

if foo, do:
    new_func()
```

# Spread assignment

In a decomposing assignment, putting `...` in front of a variable expands all remaining elements into that variable. This is called expansion assignment.

``` erg
[x,...y] = [1, 2, 3]
assert x == 1
assert y == [2, 3]
x, ...y = (1, 2, 3)
assert x == 1
assert y == (2, 3)
```

## Extract assignment

If nothing is written after `...`, the remaining elements are ignored and assigned. This type of expansion assignment is specifically called extractive assignment.
Extraction assignment is a convenient syntax for localizing specific attributes within a module or record.

``` erg
{sin; cos; tan; ..} = import "math"
```

After that, you can use `sin, cos, tan` locally.

You can do the same with records.

``` erg
record = {x = 1; y = 2}
{x; y; ...} = record
```

If you want to expand all, use `{*} = record`. It is `open` in OCaml.

``` erg
record = {x = 1; y = 2}
{*} = records
assert x == 1 and y == 2
```

<p align='center'>
    <a href='./27_comprehension.md'>Previous</a> | <a href='./29_decorator.md'>Next</a>
</p>
# Subscript (index access)

`[]` is different from normal methods.

```python
a = [!1, !2]
a[0].inc!()
assert a == [2, 2]
```

Recall that the return value of a subroutine cannot be a reference.
The type of `a[0]` here should clearly be `Ref!(Int!)` (the type of `a[0]` depends on the context).
So `[]` is actually part of a special syntax, just like `.`. Unlike Python, it cannot be overloaded.
It is also not possible to reproduce the behavior of `[]` in a method.

```python
C = Class {i = Int!}
C.steal(self) =
    self::i
```

```python,compile_fail
C.get(ref self) =
    self::i # TypeError: `self::i` is `Int!` (require ownership) but `get` doesn't own `self`
```

```python
# OK (assigning)
c = C.new({i = 1})
i = c.steal()
i.inc!()
assert i == 2
# or (own_do!)
own_do! C.new({i = 1}).steal(), i => i.inc!()
```

```python
# NG
C.new({i = 1}).steal().inc!() # OwnershipWarning: `C.new({i = 1}).steal()` is not owned by anyone
# hint: assign to a variable or use `uwn_do!`
```

Also, `[]` can be disowned, but the element is not shifted.

```python,compile_fail
a = [!1, !2]
i = a[0]
i.inc!()
assert a[1] == 2
a[0] # OwnershipError: `a[0]` is moved to `i`
```

<p align='center'>
    <a href='./11_dict.md'>Previous</a> | <a href='./13_tuple.md'>Next</a>
</p>

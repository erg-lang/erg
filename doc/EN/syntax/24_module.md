# module

Erg allows you to think of the file itself as a single record. This is called a module.

```python: foo.er
# foo.er
.i = 1
```

```python
# Defining the foo module is almost the same as defining this record
foo = {.i = 1}
```

```python: bar.er
#bar.er
foo = import "foo"
print! foo # <module 'foo'>
assert foo.i == 1
```

Since module types are also record types, deconstruction assignment is possible.

```python
{sin; cos; ...} = import "math"
```

## module visibility

```console
└─┬ ./src
   ├─ lib.er
   ├─ foo.er
   ├─bar.er
   └─┬ bar
     ├─ baz.er
     └─ qux.er
```

<p align='center'>
     <a href='./23_closure.md'>Previous</a> | <a href='./25_object_system.md'>Next</a>
</p>
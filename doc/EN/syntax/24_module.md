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

## circular dependencies

Erg allows you to define circular dependencies between modules.

```python
# foo.er
bar = import "bar"

print! bar.g 1
.f x = x
```

```python
# bar.er
foo = import "foo"

print! foo.f 1
.g x = x
```

However, variables created by procedure calls cannot be defined in circular reference modules.
This is because Erg rearranges the order of definitions according to dependencies.

```python
# foo.er
bar = import "bar"

print! bar.x
.x = g!(1) # ModuleError: variables created by procedure calls cannot be defined in circular reference modules
```

```python
# bar.er
foo = import "foo"

print! foo.x
.x = 0
```

<p align='center'>
     <a href='./23_closure.md'>Previous</a> | <a href='./25_object_system.md'>Next</a>
</p>

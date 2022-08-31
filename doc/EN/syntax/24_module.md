# module

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/24_module.md%26commit_hash%3D21e8145e83fb54ed77e7631deeee8a7e39b028a3)
](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/24_module.md&commit_hash=21e8145e83fb54ed77e7631deeee8a7e39b028a3)

Erg allows you to think of the file itself as a single record. This is called a module.

```erg: foo.er
# foo.er
.i = 1
```

``` erg
# Defining the foo module is almost the same as defining this record
foo = {.i = 1}
```

```erg: bar.er
#bar.er
foo = import "foo"
print! foo # <module 'foo'>
assert foo.i == 1
```

Since module types are also record types, deconstruction assignment is possible.

``` erg
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

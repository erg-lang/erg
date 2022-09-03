# Record

Class to which the record belongs. For example, `{i = 1}` is an element of type `Structural {i = Int}`, and is an instance of the `{i = Int}` class.
Note that instances of other classes are elements of the record type but not instances of the record class.

``` erg
assert not Structural({i = Int}) in Class
assert {i = Int} in Class

C = Class {i = Int}
c = C. new {i = 1}
assert c in Structural {i = Int}
assert not c in {i = Int}
```
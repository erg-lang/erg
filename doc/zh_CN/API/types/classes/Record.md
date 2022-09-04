# Record

记录所属的类。例如，`{i = 1}` 是`Structural {i = Int}` 类型的元素，并且是`{i = Int}` 类的实例
请注意，其他类的实例是记录类型的元素，而不是记录类的实例

```erg
assert not Structural({i = Int}) in Class
assert {i = Int} in Class

C = Class {i = Int}
c = C.new {i = 1}
assert c in Structural {i = Int}
assert not c in {i = Int}
```

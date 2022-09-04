# Record

記錄所屬的類。例如，`{i = 1}` 是`Structural {i = Int}` 類型的元素，並且是`{i = Int}` 類的實例
請注意，其他類的實例是記錄類型的元素，而不是記錄類的實例

```erg
assert not Structural({i = Int}) in Class
assert {i = Int} in Class

C = Class {i = Int}
c = C.new {i = 1}
assert c in Structural {i = Int}
assert not c in {i = Int}
```
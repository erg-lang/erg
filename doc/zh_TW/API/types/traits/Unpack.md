# Unpack

標記性狀。實現時，元素可以像記錄一樣通過模式匹配來分解

```python
C = Class {i = Int}, Impl=Unpack
C.new i = Self::new {i;}
{i} = C.new(1)
D = Class C or Int
log match D.new(1):
    (i: Int) -> i
    ({i}: C) -> i
```

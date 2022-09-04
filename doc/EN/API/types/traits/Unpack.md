# Unpack

marker trait. When implemented, elements can be decomposed by pattern matching like records.

```python
C = Class {i = Int}, Impl = Unpack
C.new i = Self::new {i;}
{i} = C.new(1)
D = Class C or Int
log match D.new(1):
     (i: Int) -> i
     ({i}: C) -> i
```
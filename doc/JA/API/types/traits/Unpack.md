# Unpack

マーカートレイト。実装すると、レコードのようにパターンマッチで要素を分解できる。

```erg
C = Class {i = Int}, Impl=Unpack
C.new i = Self::new {i;}
{i} = C.new(1)
D = Class C or Int
log match D.new(1):
    (i: Int) -> i
    ({i}: C) -> i
```

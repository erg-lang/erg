# Unpack

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/types/traits/Unpack.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/types/traits/Unpack.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

マーカートレイト。実装すると、レコードのようにパターンマッチで要素を分解できる。

```python
C = Class {i = Int}, Impl=Unpack
C.new i = Self::new {i;}
{i} = C.new(1)
D = Class C or Int
log match D.new(1):
    (i: Int) -> i
    ({i}: C) -> i
```

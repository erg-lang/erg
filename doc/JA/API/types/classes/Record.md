# Record

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/types/classes/Record.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/types/classes/Record.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

レコードの属するクラス。例えば`{i = 1}`は`Structural {i = Int}`型などの要素であり、`{i = Int}`クラスのインスタンスである。
他のクラスのインスタンスはレコード型の要素であってもレコードクラスのインスタンスではないことに注意。

```python
assert not Structural({i = Int}) in Class
assert {i = Int} in Class

C = Class {i = Int}
c = C.new {i = 1}
assert c in Structural {i = Int}
assert not c in {i = Int}
```

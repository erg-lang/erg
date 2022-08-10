# Record

レコードの属するクラス。例えば`{i = 1}`は`Structural {i = Int}`型などの要素であり、`{i = Int}`クラスのインスタンスである。
他のクラスのインスタンスはレコード型の要素であってもレコードクラスのインスタンスではないことに注意。

```erg
assert not Structural({i = Int}) in Class
assert {i = Int} in Class

C = Class {i = Int}
c = C.new {i = 1}
assert c in Structural {i = Int}
assert not c in {i = Int}
```

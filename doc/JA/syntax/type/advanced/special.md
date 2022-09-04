# 特殊型(Self, Super)

`Self`は自身の型を表します。単にエイリアスとして使うことも出来ますが、派生型中では意味が変わる(自身の型を指す)ので注意してください。

```python
@Inheritable
C = Class()
C.
    new_self() = Self.new()
    new_c() = C.new()
D = Inherit C

classof D.new_self() # D
classof D.new_c() # C
```

`Super`は基底クラスの型を表します。メソッド自体は基底クラスのものを参照しますが、インスタンスは自身の型を使います。

```python
@Inheritable
C = Class()

D = Inherit(C)
D.
    new_super() = Super.new()
    new_c() = C.new()

classof D.new_super() # D
classof D.new_c() # C
```

## 特殊型変数

`Self`, `Super`は、構造型・トレイト中では型変数として使用できます。これは、その型のサブタイプであるところのクラスを指します。すなわち、型`T`中で`Self`は`Self <: T`を意味します。

```python
Add R = Trait {
    .AddO = Type
    .`_+_`: Self, R -> Self.AddO
}
ClosedAdd = Subsume Add(Self)

ClosedAddForInt = Patch(Int, Impl := ClosedAdd)
ClosedAddForInt.
    AddO = Int

assert 1 in Add(Int, Int)
assert 1 in ClosedAdd
assert Int < Add(Int, Int)
assert Int < ClosedAdd
```

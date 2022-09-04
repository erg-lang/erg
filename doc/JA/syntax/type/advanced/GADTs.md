# 一般化代数的データ型(Generalized Algebraic Data Types, GADTs)

ErgはOr型をクラス化することで一般化代数的データ型(GADTs)を作成出来ます。

```python
Nil T = Class(Impl := Phantom T)
Cons T = Class {head = T; rest = List T}, Impl := Unpack
List T: Type = Class(Nil T or Cons T)
List.
    nil|T|() = Self(T).new Nil(T).new()
    cons head, rest | T = Self(T).new Cons(T).new(head, rest)
    head self = match self:
        {head; ...}: Cons _ -> head
        _: Nil -> panic "empty list"
{nil; cons; ...} = List

print! cons(1, cons(2, nil())).head() # 1
print! nil.head() # RuntimeError: "empty list"
```

`List(T).nil() = ...`ではなく`List.nil|T|() = ...`としているのは、使用時に型指定が不要になるからです。

```python
i = List.nil()
_: List Int = cons 1, i
```

ここで定義した`List T`はGADTsですが、素朴な実装であり、GADTsの真価を発揮していません。
例えば、上の`.head`メソッドはもし中身が空なら実行時エラーを出しますが、この検査はコンパイル時に行うことができます。

```python
List: (Type, {"Empty", "Nonempty"}) -> Type
List T, "Empty" = Class(Impl := Phantom T)
List T, "Nonempty" = Class {head = T; rest = List(T, _)}, Impl := Unpack
List.
    nil|T|() = Self(T, "Empty").new Nil(T).new()
    cons head, rest | T = Self(T, "Nonempty").new {head; rest}
List(T, "Nonempty").
    head {head; ...} = head
{nil; cons; ...} = List

print! cons(1, cons(2, nil())).head() # 1
print! nil().head() # TypeError
```

巷でよく説明されるGADTsの例は、以上のように中身が空か否か型で判定できるリストです。
Ergではさらに精密化して、長さを持つリストを定義できます。

```python
List: (Type, Nat) -> Type
List T, 0 = Class(Impl := Phantom T)
List T, N = Class {head = T; rest = List(T, N-1)}, Impl := Unpack
List.
    nil|T|() = Self(T, 0).new Nil(T).new()
    cons head, rest | T, N = Self(T, N).new {head; rest}
List(_, N | N >= 1).
    head {head; ...} = head
List(_, N | N >= 2).
    pair {head = first; rest = {head = second; ...}} = [first, second]
{nil; cons; ...} = List

print! cons(1, cons(2, nil)).pair() # [1, 2]
print! cons(1, nil).pair() # TypeError
print! cons(1, nil).head() # 1
print! nil.head() # TypeError
```

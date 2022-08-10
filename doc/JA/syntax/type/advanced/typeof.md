# Typeof, classof

`Typeof`はErgの型推論システムを覗くことができる関数であり、その挙動は複雑である。

```erg
assert Typeof(1) == {I: Int | I == 1}
i: 1..3 or 5..10 = ...
assert Typeof(i) == {I: Int | (I >= 1 and I <= 3) or (I >= 5 and I <= 10)}

C = Class {i = Int}
I = C.new {i = 1}
assert Typeof(I) == {X: C | X == I}
J: C = ...
assert Typeof(J) == {i = Int}

assert {X: C | X == I} < C and C <= {i = Int}
```

`Typeof`関数ではオブジェクトのクラスではなく構造型が返される。
なので、`C = Class T`なるクラスのインスタンス`I: C`に対しては`Typeof(I) == T`となる。
値クラスに関しては本来対応するレコード型が存在しない。この問題を解消するため、値クラスは`__valueclass_tag__`属性を持っているレコード型ということになっている。
なお、この属性にアクセスすることはできず、ユーザー定義型で`__valueclass_tag__`属性を定義することもできない。

```erg
i: Int = ...
assert Typeof(i) == {__valueclass_tag__ = Phantom Int}
s: Str = ...
assert Typeof(s) == {__valueclass_tag__ = Phantom Str}
```

`Typeof`で出力されるのは構造型のみである。構造型には属性型と篩型、(真の)代数演算型があると説明した。
これらは独立な型(推論の優先順位が存在する)であり、推論の重解は発生しない。
属性型、代数演算型は複数のクラスにまたがる可能性があるが、篩型は単一のクラスのサブタイプである。
Ergは可能な限りオブジェクトの型を篩型として推論し、それができなくなった際は篩型のベースクラスを構造化(後述)した型に拡大する。

## 構造化

すべてのクラスは構造型に変換することができる。これを __構造化__ という。クラスの構造化された型は`Structure`関数で取得できる。
クラスが`C = Class T`で定義されているとき(すべてのクラスはこの形式で定義されている)、`Structure(C) == T`になる。

```erg
C = Class {i = Int}
assert Structure(C) == {i = Int}
D = Inherit C
assert Structure(D) == {i = Int}
Nat = Class {I: Int | I >= 0}
assert Structure(Nat) == {I: Int | I >= 0}
Option T = Class (T or NoneType)
assert Structure(Option Int) == Or(Int, NoneType)
assert Structure(Option) # TypeError: only monomorphized types can be structurized
# 実際には__valueclass_tag__を持つレコードは定義できないが、概念上はこうなる
assert Structure(Int) == {__valueclass_tag__ = Phantom Int}
assert Structure(Str) == {__valueclass_tag__ = Phantom Str}
assert Structure((Nat, Nat)) == {__valueclass_tag__ = Phantom(Tuple(Nat, Nat))}
assert Structure(Nat -> Nat) == {__valueclass_tag__ = Phantom(Func(Nat, Nat))}
# マーカークラスも__valueclass_tag__を持つレコード型になる
M = Inherit Marker
assert Structure(M) == {__valueclass_tag__ = Phantom M}
D = Inherit(C and M)
assert Structure(D) == {i = Int; __valueclass_tag__ = Phantom M}
E = Inherit(Int and M)
assert Structure(E) == {__valueclass_tag__ = Phantom(And(Int, M))}
F = Inherit(E not M)
assert Structure(F) == {__valueclass_tag__ = Phantom Int}
```

# 依存型

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/14_dependent.md%26commit_hash%3D00682a94603fed2b531898200a79f2b4a64d5aae)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/14_dependent.md&commit_hash=00682a94603fed2b531898200a79f2b4a64d5aae)

依存型はErgの最大の特徴とも言っても良い機能です。
依存型とは、値を引数に取る型です。通常の多相型は型のみを引数に取れますが、その制限を緩めたのが依存型といえます。

依存型は、`[T; N]`(`Array(T, N)`)などがそれに相当します。
この型は、中身の型`T`だけでなく、中身の個数`N`にも依存して決まる型です。`N`には`Nat`型のオブジェクトが入ります。

```python
a1 = [1, 2, 3]
assert a1 in [Nat; 3]
a2 = [4, 5, 6, 7]
assert a1 in [Nat; 4]
assert a1 + a2 in [Nat; 7]
```

関数引数で渡した型オブジェクトが戻り値型に関連する場合は、以下のように記述します。

```python
narray: |N: Nat| {N} -> [{N}; N]
narray(N: Nat): [N; N] = [N; N]
assert narray(3) == [3, 3, 3]
```

依存型を定義する際は、型引数が全て定数でなくてはなりません。

依存型そのものは既存の言語にも存在するものですが、Ergでは依存型にプロシージャルメソッドを定義できるという特徴があります。

```python
x = 1
f x =
    print! f::x, module::x

# Phantom型は型引数と同じ値になるPhantomという属性を持つ
T X: Int = Class Impl := Phantom X
T(X).
    x self = self::Phantom

T(1).x() # 1
```

可変依存型の型引数はメソッドの適用によって遷移させることができます。
遷移指定は`~>`で行います。

```python
# `Id`は不変型なので遷移させることはできないことに注意する
VM!(State: {"stopped", "running"}! := _, Id: Nat := _) = Class(..., Impl := Phantom! State)
VM!().
    # 変わらない変数は`_`を渡せば省略可能, デフォルト引数にしておけば書く必要すらない
    start! ref! self("stopped" ~> "running") =
        self.initialize_something!()
        self::set_phantom!("running")

# 型引数ごとに切り出すこともできる(定義されたモジュール内でのみ)
VM!.new() = VM!(!"stopped", 1).new()
VM!("running" ~> "running").stop! ref! self =
    self.close_something!()
    self::set_phantom!("stopped")

vm = VM!.new()
vm.start!()
vm.stop!()
vm.stop!() # TypeError: VM!(!"stopped", 1) doesn't have .stop!()
# hint: VM!(!"running", 1) has .stop!()
```

既存の型を組み込んだり継承して依存型を作ることもできます。

```python
MyArray(T, N) = Inherit [T; N]

# .arrayと連動してself: Self(T, N)の型が変わる
MyStruct!(T, N: Nat!) = Class {.array: [T; !N]}
```

## 実体指定

動的配列`arr: [T; !N]`について、処理を進めていくうちに`N`の情報が失われてしまったとします。
この情報は`assert arr.__len__() == X`とすることで回復させることができます。

```erg
arr: [Int; !_]
assert arr.__len__() == 3
arr: [Int; !3]
```

これは型パラメータの __実体指定__ によって可能となっています。配列型`Array(T, N)`は以下のように定義されています。

```erg
Array T <-> Union Self.map(x -> Typeof x), N <-> Self.__len__() = ...
```

`<->`は依存型のパラメータのみで使える特別な記号で、そのパラメータに対する実体を指示します。実体であるところの右辺式は、コンパイル時に計算可能でなくても構いません。コンパイル時情報である`N`と実行時情報である`Self.__len__()`が実体指定を通してリンクされる訳です。
実体指定に沿った方法でassertionが行われると、型パラメータの情報が復活します。すなわち、`assert arr.__len__() == N`とすると`N`の情報が復活します。ただしこの場合の`N`はコンパイル時計算可能でなくてはなりません。
実体指定は`assert`以外に`match`でも活用されます。

```erg
arr: [Obj; _]
match! arr:
    pair: [Obj; 2] => ...
    ints: [Int; _] => ...
    _ => ...
```

<p align='center'>
    <a href='./13_algebraic.md'>Previous</a> | <a href='./15_quantified.md'>Next</a>
</p>

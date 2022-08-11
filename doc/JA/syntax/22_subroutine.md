# Subroutine signatures

## Func

```erg
some_func(x: T, y: U) -> V
some_func: (T, U) -> V
```

## Proc

```erg
some_proc!(x: T, y: U) => V
some_proc!: (T, U) => V
```

## Func Method

メソッド型は、外部からは`Self`で指定できない。

```erg
.some_method(self, x: T, y: U) => ()
# Self.(T, U) => ()はselfの所有権を奪う
.some_method: Ref(Self).(T, U) => ()
```

## Proc Method (dependent)

以下で、型`T!`は`N: Nat`という型引数を取るとする。外部から指定する場合は型変数を使用する。

```erg
T!: Nat -> Type
# ~>は適用前後の型引数の状態を示す(このときselfは可変参照でなくてはならない)
T!(N).some_method!: (Ref! T!(N ~> N+X), X: Nat) => ()
```

注意として、`.some_method`の型は`|N, X: Nat| Ref!(T(N ~> N+X)).({X}) => ()`となる。
`ref!`がついていない、すなわち適用後所有権が奪われるメソッドでは、型引数の遷移(`~>`)を使用できない。

所有権が奪われる場合は以下のようになる。

```erg
# Nを使わないなら_で省略可
# .some_method!: |N, X: Nat| T!(N).({X}) => T!(N+X)
.some_method!|N, X: Nat|(self(N), X: Nat) => T!(N+X)
```

## Operator

``で囲むことで通常の関数と同じように定義できる。
`and`や`or`などの中置アルファベット演算子は囲むことで中置演算子として定義できる。

```erg
and(x, y, z) = x and y and z
`_+_`(x: Foo, y: Foo) = x.a + y.a
`-_`(x: Foo) = Foo.new(-x.a)
```

<p align='center'>
    <a href='./21_lambda.md'>Previous</a> | <a href='./23_scope.md'>Next</a>
</p>

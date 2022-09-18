# サブルーチンシグネチャ

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/22_subroutine.md%26commit_hash%3Dddcbe9859e56d63bb3dd001be216cd4343e1771e)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/22_subroutine.md&commit_hash=ddcbe9859e56d63bb3dd001be216cd4343e1771e)

## 関数

```python
some_func(x: T, y: U) -> V
some_func: (T, U) -> V
```

## プロシージャ

```python
some_proc!(x: T, y: U) => V
some_proc!: (T, U) => V
```

## 関数メソッド

メソッド型は、外部からは`Self`で指定できません。

```python
.some_method(self, x: T, y: U) => ()
# Self.(T, U) => ()はselfの所有権を奪う
.some_method: (Ref Self, T, U) => ()
```

## プロシージャメソッド(依存)

以下で、型`T!`は`N: Nat`という型引数を取るとします。外部から指定する場合は型変数を使用します。

```python
T!: Nat -> Type
# ~>は適用前後の型引数の状態を示す(このときselfは可変参照でなくてはならない)
T!(N).some_method!: (self: Ref!(T! N ~> N+X), X: Nat) => ()
```

注意として、`.some_method`の型は`|N, X: Nat| (self: Ref!(T! N ~> N+X), {X}) => ()`となります。
`ref!`がついていない、すなわち適用後所有権が奪われるメソッドでは、型引数の遷移(`~>`)を使用できません。

所有権が奪われる場合は以下のようになります。

```python
# Nを使用しないならば_で省略できる
# .some_method!: |N, X: Nat| (T!(N+X), {X}) => T!(N+X)
.some_method!|N, X: Nat|(self: T!(N), X: Nat) => T!(N+X)
```

## 演算子

``で囲むことで通常の関数と同じように定義できます。
`and`や`or`などの中置アルファベット演算子は囲むことで中置演算子として定義できます。

```python
and(x, y, z) = x and y and z
`_+_`(x: Foo, y: Foo) = x.a + y.a
`-_`(x: Foo) = Foo.new(-x.a)
```

<p align='center'>
    <a href='./21_lambda.md'>Previous</a> | <a href='./23_closure.md'>Next</a>
</p>

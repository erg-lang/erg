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

不能从外部指定方法类型。


```erg
.some_method(self, x: T, y: U) => ()
# Self.(T, U) => ()はselfの所有権を奪う
.some_method: Ref(Self).(T, U) => ()
```

## Proc Method (dependent)

下面，假定类型采用类型参数<gtr=“9”/>。如果从外部指定，请使用类型变量。


```erg
T!: Nat -> Type
# ~>は適用前後の型引数の状態を示す(このときselfは可変参照でなくてはならない)
T!(N).some_method!: (Ref! T!(N ~> N+X), X: Nat) => ()
```

请注意，的类型为<gtr=“11”/>。类型参数转换（<gtr=“13”/>）不适用于没有<gtr=“12”/>的方法，即应用后将被剥夺所有权。

所有权被剥夺的情况如下。


```erg
# Nを使わないなら_で省略可
# .some_method!: |N, X: Nat| T!(N).({X}) => T!(N+X)
.some_method!|N, X: Nat|(self(N), X: Nat) => T!(N+X)
```

## Operator

用括起来，可以像定义常规函数一样定义函数。可以将<gtr=“15”/>和<gtr=“16”/>等中置字母运算符括起来，将其定义为中置运算符。


```erg
and(x, y, z) = x and y and z
`_+_`(x: Foo, y: Foo) = x.a + y.a
`-_`(x: Foo) = Foo.new(-x.a)
```

<p align='center'>
    <a href='./21_lambda.md'>Previous</a> | <a href='./23_closure.md'>Next</a>
</p>

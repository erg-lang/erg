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

不能從外部指定方法類型。


```erg
.some_method(self, x: T, y: U) => ()
# Self.(T, U) => ()はselfの所有権を奪う
.some_method: Ref(Self).(T, U) => ()
```

## Proc Method (dependent)

下面，假定類型採用類型參數<gtr=“9”/>。如果從外部指定，請使用類型變量。


```erg
T!: Nat -> Type
# ~>は適用前後の型引數の狀態を示す(このときselfは可変參照でなくてはならない)
T!(N).some_method!: (Ref! T!(N ~> N+X), X: Nat) => ()
```

請注意，的類型為<gtr=“11”/>。類型參數轉換（<gtr=“13”/>）不適用於沒有<gtr=“12”/>的方法，即應用後將被剝奪所有權。

所有權被剝奪的情況如下。


```erg
# Nを使わないなら_で省略可
# .some_method!: |N, X: Nat| T!(N).({X}) => T!(N+X)
.some_method!|N, X: Nat|(self(N), X: Nat) => T!(N+X)
```

## Operator

用括起來，可以像定義常規函數一樣定義函數。可以將<gtr=“15”/>和<gtr=“16”/>等中置字母運算符括起來，將其定義為中置運算符。


```erg
and(x, y, z) = x and y and z
`_+_`(x: Foo, y: Foo) = x.a + y.a
`-_`(x: Foo) = Foo.new(-x.a)
```

<p align='center'>
    <a href='./21_lambda.md'>Previous</a> | <a href='./23_closure.md'>Next</a>
</p>
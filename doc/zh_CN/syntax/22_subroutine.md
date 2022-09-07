# 子程序签名

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/22_subroutine.md%26commit_hash%3D51de3c9d5a9074241f55c043b9951b384836b258)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/22_subroutine.md&commit_hash=51de3c9d5a9074241f55c043b9951b384836b258)

## 函数

```python
some_func(x: T, y: U) -> V
some_func: (T, U) -> V
```

## 过程

```python
some_proc!(x: T, y: U) => V
some_proc!: (T, U) => V
```

## 函数方法

方法类型不能用`Self`在外部指定

```python
.some_method(self, x: T, y: U) => ()
# (Self, T, U) => () 拥有 self 的所有权
.some_method: (Ref(Self), T, U) => ()
```

## 过程方法(依赖)

在下文中，假设类型 `T!` 采用类型参数 `N: Nat`。 要在外部指定它，请使用类型变量

```python
T!: Nat -> Type
# ~> 表示应用前后类型参数的状态(此时self必须是变量引用)
T!(N).some_method!: (Ref!(T! N ~> N+X), X: Nat) => ()
```

注意，`.some_method` 的类型是 `| N，X：Nat| (Ref!(T! N ~> N+X), {X}) => ()`。
对于没有 `ref!` 的方法，即在应用后被剥夺所有权，不能使用类型参数转换(`~>`)。

如果取得所有权，则如下所示。

```python
# 如果不使用N，可以用_省略。
# .some_method!: |N, X: Nat| (T!(N), {X}) => T!(N+X)
.some_method!|N, X: Nat| (self: T!(N), X: Nat) => T!(N+X)
```

## 运算符

可以通过用 ` 括起来将其定义为普通函数。

中性字母运算符，例如 `and` 和 `or` 可以通过用 ` 括起来定义为中性运算符。

```python
and(x, y, z) = x and y and z
`_+_`(x: Foo, y: Foo) = x.a + y.a
`-_`(x: Foo) = Foo.new(-x.a)
```

<p align='center'>
    <a href='./21_lambda.md'>上一页</a> | <a href='./23_closure.md'>下一页</a>
</p>

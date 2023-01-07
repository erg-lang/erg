# 子程序

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/23_subroutine.md%26commit_hash%3De959b3e54bfa8cee4929743b0193a129e7525c61)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/23_subroutine.md&commit_hash=e959b3e54bfa8cee4929743b0193a129e7525c61)

## 函數

```python,checker_ignore
some_func(x: T, y: U) -> V
some_func: (T, U) -> V
```

## 過程

```python,checker_ignore
some_proc!(x: T, y: U) => V
some_proc!: (T, U) => V
```

## 函數方法

方法類型不能用`Self`在外部指定

```python,checker_ignore
.some_method(self, x: T, y: U) => ()
# (Self, T, U) => () 擁有 self 的所有權
.some_method: (Ref(Self), T, U) => ()
```

## 過程方法(依賴)

在下文中，假設類型 `T!` 采用類型參數 `N: Nat`。要在外部指定它，請使用類型變量

```python
K!: Nat -> Type
# ~> 表示應用前后類型參數的狀態(此時self必須是變量引用)
K!(N).some_method!: (Ref!(K! N ~> N+X), X: Nat) => ()
```

注意，`.some_method` 的類型是 `| N，X: Nat| (Ref!(K! N ~> N+X), {X}) => ()`
對于沒有 `ref!` 的方法，即在應用后被剝奪所有權，不能使用類型參數轉換(`~>`)

如果取得所有權，則如下所示

```python
# 如果不使用N，可以用_省略
# .some_method!: |N, X: Nat| (T!(N), {X}) => T!(N+X)
.some_method!|N, X: Nat| (self: T!(N), X: Nat) => T!(N+X)
```

## 運算符

可以通過用 ` 括起來將其定義為普通函數

中性字母運算符，例如 `and` 和 `or` 可以通過用 ` 括起來定義為中性運算符

```python
and(x, y, z) = x and y and z
`_+_`(x: Foo, y: Foo) = x.a + y.a
`-_`(x: Foo) = Foo.new(-x.a)
```

<p align='center'>
    <a href='./22_lambda.md'>上一頁</a> | <a href='./24_closure.md'>下一頁</a>
</p>

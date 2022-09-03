# Never

它是所有类型的子类型。 它是一个`Class`，因为它拥有所有的方法，当然还有 `.new`。但是，它没有实例，并且Erg会在即将创建的那一刻停止。
还有一种叫做`Panic`的类型没有实例，但是`Never`用于正常终止或故意无限循环，`Panic`用于异常终止。

```erg
# Never <: Panic
f(): Panic = exit 0 # OK
g(): Never = panic() # TypeError
```

`Never`/`Panic`的 OR 类型，例如`T 或 Never`可以转换为`T`。 这是因为`Never`在语义上是一个从不出现的选项（如果出现了，程序会立即停止）。
但是，在函数的返回值类型中使用时，`or Never`不能省略，因为它表示程序可能会终止。
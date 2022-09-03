# 关键字参数函数类型


```erg
h(f) = f(y: 1, x: 2)
h: |T: Type|((y: Int, x: Int) -> T) -> T
```

带关键字自变量函数的部分定型规则如下所示。


```erg
((x: T, y: U) -> V) <: ((T, U) -> V)  # x, y are arbitrary keyword parameters
((y: U, x: T) -> V) <: ((x: T, y: U) -> V)
((x: T, y: U) -> V) <: ((y: U, x: T) -> V)
```

这意味着关键词自变量可以删除或者替换。但是，两者不能同时进行。也就是说，不能将转换为<gtr=“5”/>。另外，带有关键词自变量的只在顶级元组内，排列和嵌套的元组中不带有关键词自变量。


```erg
Valid: [T, U] -> V
Invalid: [x: T, y: U] -> V
Valid: (x: T, ys: (U,)) -> V
Invalid: (x: T, ys: (y: U,)) -> V
```

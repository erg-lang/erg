# 发电机

生成器是在块中使用过程的特殊过程。


```erg
g!() =
    yield! 1
    yield! 2
    yield! 3
```

是在子例程块中定义的过程，它调用<gtr=“6”/>。它返回的返回值类似于<gtr=“7”/>，但它保存块在该时刻的执行状态，并在再次调用时继续执行。生成器既是过程又是迭代器。Python 生成器是生成迭代器的函数，而 Erg 直接迭代。过程本身通常不是可变对象（没有<gtr=“8”/>），但生成器是可变的，因为它可以在每次执行时更改其内容。


```erg
# Generator! < Proc
g!: Generator!((), Int)
assert g!() == 1
assert g!() == 2
assert g!() == 3
```

可以按如下方式定义 Python 样式生成器。


```erg
make_g() = () =>
    yield! 1
    yield! 2
    yield! 3
make_g: () => Generator!((), Int)
```

<p align='center'>
    <a href='./33_package_system.md'>Previous</a> | Next
</p>

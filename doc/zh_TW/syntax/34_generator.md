# 發電機

生成器是在塊中使用過程的特殊過程。


```erg
g!() =
    yield! 1
    yield! 2
    yield! 3
```

是在子例程塊中定義的過程，它調用<gtr=“6”/>。它返回的返回值類似於<gtr=“7”/>，但它保存塊在該時刻的執行狀態，並在再次調用時繼續執行。生成器既是過程又是迭代器。 Python 生成器是生成迭代器的函數，而 Erg 直接迭代。過程本身通常不是可變對象（沒有<gtr=“8”/>），但生成器是可變的，因為它可以在每次執行時更改其內容。


```erg
# Generator! < Proc
g!: Generator!((), Int)
assert g!() == 1
assert g!() == 2
assert g!() == 3
```

可以按如下方式定義 Python 樣式生成器。


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
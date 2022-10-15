# 閉包

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/23_closure.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/23_closure.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

Erg 子例程有一個稱為"閉包"的功能，可以捕獲外部變量

```python
outer = 1
f x = outer + x
assert f(1) == 2
```

與不可變對象一樣，可變對象也可以被捕獲

```python
sum = !0
for! 1..10, i =>
    sum.add!i
assert sum == 45

p!x=
    sum.add!x
p!(1)
assert sum == 46
```

但是請注意，函數不能捕獲可變對象
如果可以在函數中引用可變對象，則可以編寫如下代碼

```python
# !!! 這段代碼實際上給出了一個錯誤！！！
i = !0
f x = i + x
assert f 1 == 1
i.add! 1
assert f 1 == 2
```

該函數應該為相同的參數返回相同的值，但假設被打破了
請注意，`i` 僅在調用時進行評估

如果您想在定義函數時獲取可變對象的內容，請調用`.clone`

```python
i = !0
immut_i = i.clone().freeze()
fx = immut_i + x
assert f 1 == 1
i.add! 1
assert f 1 == 1
```

## avoid mutable state, functional programming

```python
# Erg
sum = !0
for! 1..10, i =>
    sum.add!i
assert sum == 45
```

上面的等效程序可以用 Python 編寫如下: 

```python
# Python
sum = 0
for i in range(1, 10):
    sum += i
assert sum == 45
```

但是，Erg 建議使用更簡單的表示法
與其使用子例程和可變對象來傳遞狀態，不如使用一種使用函數來定位狀態的風格。這稱為函數式編程

```python
# 功能風格
sum = (1..10).sum()
assert sum == 45
```

上面的代碼給出了與之前完全相同的結果，但是您可以看到這個代碼要簡單得多

`fold` 函數可以用來做比 sum 更多的事情
`fold` 是一個迭代器方法，它為每次迭代執行參數 `f`
累加結果的計數器的初始值在 `init` 中指定，并在 `acc` 中累加

```python
# 從0開始，結果會
sum = (1..10).fold(init: 0, f: (acc, i) -> acc + i)
assert sum == 45
```

Erg 被設計為對使用不可變對象進行編程的自然簡潔描述

<p align='center'>
    <a href='./22_subroutine.md'>上一頁</a> | <a href='./24_module.md'>下一頁</a>
</p>
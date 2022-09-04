# 與 Python 合作

## 導出到 Python

編譯 Erg 腳本將生成一個.pyc 文件，你可以將其作為一個模塊導入 Python。但是，在 Erg 端設置為私有的變量不能從 Python 訪問。


```erg
# foo.er
.public = "this is a public variable"
private = "this is a private variable"
```


```console
erg --compile foo.er
```


```python
import foo

print(foo.public)
print(foo.private) # AttributeError:
```

## 從 Python 導入

默認情況下，從 Python 引入的所有對像都是類型。長此以往，我們也無法進行比較，所以我們需要進行類型的篩選。

## 標準庫類型

Python 標準庫中的所有 API 都由 Erg 開發團隊指定類型。


```erg
time = pyimport "time"
time.sleep! 1
```

## 指定用戶腳本類型

創建一個文件，為 Python 的<gtr=“10”/>模塊創建類型。 Python 端的 type hint 不是 100% 的保證，因此將被忽略。


```python
# foo.py
X = ...
def bar(x):
    ...
def baz():
    ...
```


```erg
# foo.d.er
foo = pyimport "foo"
.X = declare foo.'X', Int
.bar = declare foo.'bar', Int -> Int
.baz! = declare foo.'baz', () => Int
```


```erg
foo = pyimport "foo"
assert foo.bar(1) in Int
```

它通過在運行時執行類型檢查來保證類型安全性。函數的工作原理大致如下。


```erg
declare|S: Subroutine| sub!: S, T =
    # 実は、=>はブロックの副作用がなければ関數にキャストできる
    x =>
        assert x in T.Input
        y = sub!(x)
        assert y in T.Output
        y
```

這是一個運行時開銷，因此計劃在 Erg 類型系統上對 Python 腳本進行靜態類型分析。

<p align='center'>
    <a href='./31_pipeline.md'>Previous</a> | <a href='./33_package_system.md'>Next</a>
</p>
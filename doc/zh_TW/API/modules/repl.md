# 模塊`repl`

提供REPL(Read-Eval-Print-Loop)相關的API。

## 功能

* `gui_help`

在瀏覽器中查看有關對象的信息。 可以離線使用。

## 類型

### 猜測 = 對象

#### 方法

* `.guess`

在給定參數和返回值的情況下推斷函數。

```python
1.guess((1,), 2) # <Int.__add__ method>
[1, 2].guess((3, 4), [1, 2, 3, 4]) # <Array(T, N).concat method>
```
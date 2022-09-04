# 模块`repl`

提供REPL(Read-Eval-Print-Loop)相关的API。

## 功能

* `gui_help`

在浏览器中查看有关对象的信息。 可以离线使用。

## 类型

### 猜测 = 对象

#### 方法

* `.guess`

在给定参数和返回值的情况下推断函数。

```python
1.guess((1,), 2) # <Int.__add__ method>
[1, 2].guess((3, 4), [1, 2, 3, 4]) # <Array(T, N).concat method>
```
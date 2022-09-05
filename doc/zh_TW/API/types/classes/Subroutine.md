# Subroutine

Func和Proc的基本類型。

## 方法

* return

中斷子程序并返回指定的值。 用于快速逃離嵌套

```python
f x =
    for 0..10, i ->
        if i == 5:
            do
                f::return i
            do
                log i
```

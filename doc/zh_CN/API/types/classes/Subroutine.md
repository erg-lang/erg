# Subroutine

Func和Proc的基本类型。

## 方法

* return

中断子程序并返回指定的值。 用于快速逃离嵌套

```python
f x =
    for 0..10, i ->
        if i == 5:
            do
                f::return i
            do
                log i
```

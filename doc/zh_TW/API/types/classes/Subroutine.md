# Subroutine

Func和Proc的基本類型。

## methods

* return

中斷子程序並返回指定的值。用於快速逃離嵌套

```erg
f x =
    for 0..10, i ->
        if i == 5:
            do
                f::return i
            do
                log i
```
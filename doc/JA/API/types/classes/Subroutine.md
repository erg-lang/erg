# Subroutine

FuncやProcの基底型です。

## methods

* return

サブルーチンを中断して、指定した値を返す。ネストから一気に脱出する際に便利。

```erg
f x =
    for 0..10, i ->
        if i == 5:
            do
                f::return i
            do
                log i
```

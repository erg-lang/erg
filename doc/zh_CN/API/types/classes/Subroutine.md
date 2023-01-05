# Subroutine

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/types/classes/Subroutine.md%26commit_hash%3D14b0c449efc9e9da3e10a09c912a960ecfaf1c9d)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/types/classes/Subroutine.md&commit_hash=14b0c449efc9e9da3e10a09c912a960ecfaf1c9d)

Func和Proc的基本类型

## 方法

* return

中断子程序并返回指定的值。用于快速逃离嵌套

```python
f x =
    for 0..10, i ->
        if i == 5:
            do:
                f::return i
            do:
                log i
```

# 模块`repl`

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/API/modules/repl.md%26commit_hash%3Dc6eb78a44de48735213413b2a28569fdc10466d0)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/API/modules/repl.md&commit_hash=c6eb78a44de48735213413b2a28569fdc10466d0)

提供REPL(Read-Eval-Print-Loop)相关的API

## 功能

* `gui_help`

在浏览器中查看有关对象的信息。可以离线使用

## 类型

### 猜测 = 对象

#### 方法

* `.guess`

在给定参数和返回值的情况下推断函数

```python
1.guess((1,), 2) # <Int.__add__ method>
[1, 2].guess((3, 4), [1, 2, 3, 4]) # <List(T, N).concat method>
```

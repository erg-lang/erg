# 值類型

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/08_value.md%26commit_hash%3Db713e6f5cf9570255ccf44d14166cb2a9984f55a)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/08_value.md&commit_hash=b713e6f5cf9570255ccf44d14166cb2a9984f55a)

值類型是可以在編譯時評估的 Erg 內置類型，具體來說: 

```python
Value = (
    Int
    or Nat
    or Ratio
    or Float
    or Complex
    or Bool
    or Str
    or NoneType
    or Array Const
    or Tuple Const
    or Set Const
    or ConstFunc(Const, _)
    or ConstProc(Const, _)
    or ConstMethod(Const, _)
)
```

應用于它們的值類型對象、常量和編譯時子例程稱為 __constant 表達式__

```python
1, 1.0, 1+2im, True, None, "aaa", [1, 2, 3], Fib(12)
```

小心子程序。子例程可能是也可能不是值類型
由于子程序的實質只是一個指針，因此可以將其視為一個值[<sup id="f1">1</sup>](#1)，但是在編譯不是子程序的東西時不能使用 在恒定的上下文中。不是值類型，因為它沒有多大意義

將來可能會添加歸類為值類型的類型

---

<span id="1" style="font-size:x-small"><sup>1</sup> Erg 中的術語"值類型"與其他語言中的定義不同。純 Erg 語義中沒有內存的概念，并且因為它被放置在堆棧上而說它是值類型，或者因為它實際上是一個指針而說它不是值類型是不正確的。值類型僅表示它是"值"類型或其子類型。[?](#f1)</span>

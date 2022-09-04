# 值類型（Value types）

值類型是 Erg 內置類型中可以進行編譯時評估的類型，具體如下所示。


```erg
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

值類型的對象常量和編譯時子例程稱為常量表達式。


```erg
1, 1.0, 1+2im, True, None, "aaa", [1, 2, 3], Fib(12)
```

需要注意子程序。子程序可能是值類型，也可能不是。雖然每個子例程的實體都是一個指針，並且都是一個值，但在編譯時，在常量上下文中使用非子例程並沒有什麼意義，因此它不是一個值類型。

以後可能會添加一些類型，這些類型被歸類為值類型。

---

<span id="1" style="font-size:x-small">1<gtr=“7”/>Erg 中的值類型一詞與其他語言中的定義不同。在純 Erg 語義學中，內存的概念不存在，因為它被放置在堆棧中，所以它是一種值類型，或者因為它是一個指針，所以它不是一種值類型，這些說法是不正確的。從根本上說，值類型是<gtr=“4”/>類型或其子類型。 <gtr=“5”/></span>
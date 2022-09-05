# 値型(Value types)

値型はErg組み込み型のうちコンパイル時評価が可能な型で、具体的には以下のものです。

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

値型のオブジェクト・定数、およびそれにコンパイル時サブルーチンを適用したものを定数式と呼びます。

```python
1, 1.0, 1+2im, True, None, "aaa", [1, 2, 3], Fib(12)
```

サブルーチンについては注意が必要です。サブルーチンは値型であるものとそうでないものがあります。
サブルーチンの実体は単なるポインタであるためすべて値として扱っても良い[<sup id="f1">1</sup>](#1)のですが、コンパイル時サブルーチンでないものを定数文脈で使えてもあまり意味がないため、値型とはなっていません。

値型に分類される型は、将来的には追加される可能性があります。

---

<span id="1" style="font-size:x-small"><sup>1</sup> Ergにおける値型という用語は、他の言語での定義とは異なっています。純粋なErgの意味論内でメモリという概念は存在せず、スタックに置かれるから値型であるとか、実体としてポインタだから値型ではない、といった言明は正しくありません。あくまで、値型は`Value`型もしくはそのサブタイプであるという意味しか持ちません。 [↩](#f1)</span>

# 特殊化

Ergのトレイト特殊化は―またそれ以前に多相サブルーチンの解決はテンプレート方式を採用している[<sup id="f1">1</sup>](#1)。
この方式では全ての単相化サブルーチンがコードとして生成される。

## 例

```erg
Natural = Class Nat
Natural|<: Add(Int)|.
    Output = Int
    __add__ self, other = self::base + other
Natural|<: Add(Nat)|.
    Output = Nat
    __add__ self, other = self::base + other

_: Int = Natural.new(1) + -1
_: Nat = Natural.new(1) + 1
```

Pythonバックエンドで生成されるコードは、概念的には以下のようになる。その他のバックエンドでも同じである。

```python
class Natural(Nat):
    def __init__(self, base):
        self.base = base
    def "__add__::<Add(Int)>"(self, other):
        return self.base + other
    def "__add__::<Add(Nat)>"(self, other):
        return self.base + other

def "__add__::<Natural,Add(Int)>"(self, other):
    return self."__add__::<Add(Int)>"(other)
def "__add__::<Natural,Add(Nat)>"(self, other):
    return self."__add__::<Add(Nat)>"(other)

_: Int = "__add__::<Natural,Add(Int)>"(Natural.new(1), -1)
_: Nat = "__add__::<Natural,Add(Nat)>"(Natural.new(1), 1)
```

生成される単相化サブルーチンは、多相関数を呼ぶ関数の数の増加に従ってねずみ算式に増加する。

---

<span id="1" style="font-size:x-small"><sup>1</sup> その他の方式には型消去方式などがある。これは多相サブルーチンが実行時に実引数の型を見て処理を決める方式である。Haskellなどが採用している。Rustのdynと同じく実行時のオーバーヘッドを伴う。Ergの場合、Pythonバックエンドではどちらを採用しても大した違いはない(結局Pythonが動的型付けのため)が、ネイティブバイナリバックエンドの実装予定を鑑みてテンプレート方式を採用している[↩](#f1)</span>

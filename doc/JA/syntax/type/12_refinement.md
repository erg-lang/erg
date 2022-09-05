# 篩型(Refinement Type)


Refinement type(篩型、ふるいがた)は、述語式によって制約付けられた型です。列挙型や区間型は篩型の一種です。

篩型の標準形は`{Elem: Type | (Pred)*}`です。これは、`Pred`を満たす`Elem`を要素とする型である、という意味です。
篩型に使えるのは[Const型](./advanced/const.md)のみです。

```python
Nat = 0.._
Odd = {N: Int | N % 2 == 1}
Char = StrWithLen 1
# StrWithLen 1 == {_: StrWithLen N | N == 1}
[Int; 3] == {_: Array Int, N | N == 3}
Array3OrMore == {A: Array _, N | N >= 3}
```

複数のPredがあるとき、`;`か`and`, `or`で区切れます。`;`と`and`は同じ意味です。

`Odd`の要素は`1, 3, 5, 7, 9, ...`です。
篩にかけるように既存の型の一部を要素とする型になることから篩型と呼ばれます。

`Pred`は(左辺)述語式と呼ばれます。これは代入式と同じく意味のある値を返すものではなく、左辺にはパターンしか置けません。
すなわち、`X**2 - 5X + 6 == 0`のような式は篩型の述語式としては使えません。この点において、右辺式の述語式とは異なります。

```python
{X: Int | X**2 - 5X + 6 == 0} # SyntaxError: the predicate form is invalid. Only names can be on the left-hand side
```

あなたが二次方程式の解法を知っているならば、上の篩型は`{2, 3}`と同等になるだろうと予想できるはずです。
しかしErgコンパイラは代数学の知識をほとんど持ち合わせていないので、右の述語式を解決できないのです。

## Smart Cast

`Odd`を定義したのはいいですが、このままではリテラル以外ではあまり使えないようにみえます。通常の`Int`オブジェクトの中の奇数を`Odd`に昇格させる、つまり`Int`を`Odd`にダウンキャストするためには、`Odd`のコンストラクタを通す必要があります。
篩型の場合、通常のコンストラクタ`.new`はパニックする可能性があり、`.try_new`という`Result`型を返す補助的なコンストラクタもあります。

```python
i = Odd.new (0..10).sample!()
i: Odd # or Panic
```

また、`match`中で型指定として使用することもできます。

```python
# i: 0..10
i = (0..10).sample!()
match i:
    o: Odd ->
        log "i: Odd"
    n: Nat -> # 0..10 < Nat
        log "i: Nat"
```

ただし、Ergは現在のところ`Odd`でなかったから`Even`、などといった副次的な判断はできません。

## 列挙型、区間型と篩型

今まで紹介した列挙型と区間型は、篩型の糖衣構文です。
`{a, b, ...}`は`{I: Typeof(a) | I == a or I == b or ... }`に、`a..b`は`{I: Typeof(a) | I >= a and I <= b}`に脱糖されます。

```python
{1, 2} == {I: Int | I == 1 or I == 2}
1..10 == {I: Int | I >= 1 and I <= 10}
1..<10 == {I: Int | I >= 1 and I < 10} == {I: Int | I >= 1 and I <= 9}
```

## 篩パターン

`_: {X}`を`X`と書き換えられるように(定数パターン)、`_: {X: T | Pred}`は`X: T | Pred`と書き換えることができます。

```python
# メソッド.mは長さ3以上の配列に定義される
Array(T, N | N >= 3)
    .m(&self) = ...
```

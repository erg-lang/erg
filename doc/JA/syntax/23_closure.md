# クロージャ

Ergのサブルーチンには、外部変数を捕捉する「クロージャ」という機能があります。

```python
outer = 1
f x = outer + x
assert f(1) == 2
```

不変オブジェクトと同じく、可変オブジェクトも捕捉できます。

```python
sum = !0
for! 1..10, i =>
    sum.add! i
assert sum == 45

p! x =
    sum.add! x
p!(1)
assert sum == 46
```

しかし、関数は可変オブジェクトを捕捉できないので注意が必要です。
仮に可変オブジェクトが関数内で参照できると、以下のようなコードが書けてしまいます。

```python
# !!! このコードは実際にはエラーになる !!!
i = !0
f x = i + x
assert f 1 == 1
i.add! 1
assert f 1 == 2
```

関数は同じ引数に対して同じ値を返すべきですが、その前提が破れてしまっています。
`i`は呼び出し時に初めて評価されることに注意してください。

関数定義時点での可変オブジェクトの内容がほしい場合は`.clone`を呼び出します。

```python
i = !0
immut_i = i.clone().freeze()
f x = immut_i + x
assert f 1 == 1
i.add! 1
assert f 1 == 1
```

## 可変状態の回避、関数型プログラミング

```python
# Erg
sum = !0
for! 1..10, i =>
    sum.add! i
assert sum == 45
```

上と同等のプログラムは、Pythonでは以下のように記述できます。

```python
# Python
sum = 0
for i in range(1, 10):
    sum += i
assert sum == 45
```

しかし、Ergではもっとシンプルな書き方を推奨します。
サブルーチンと可変オブジェクトを使って状態を持ち回す代わりに、関数を使用する状態を局所化するスタイルを使います。これは関数型プログラミングと呼ばれます。

```python
# Functional style
sum = (1..10).sum()
assert sum == 45
```

上のコードは先程と全く同じ結果になりますが、こちらのほうが遥かにシンプルであることが見て取れます。

`fold`関数を使用すれば、合計以外にも多様な操作を行うことができます。
`fold`はイテレータのメソッドで、各イテレーションごとに引数`f`を実行します。
結果を蓄積するカウンタの初期値は`init`で指定し、`acc`に蓄積されていきます。

```python
# start with 0, result will
sum = (1..10).fold(init: 0, f: (acc, i) -> acc + i)
assert sum == 45
```

不変オブジェクトによるプログラミングで自然と簡潔な記述となるように、Ergは設計されています。

<p align='center'>
    <a href='./22_subroutine.md'>Previous</a> | <a href='./24_module.md'>Next</a>
</p>

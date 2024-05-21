# 所有権システム

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/20_ownership.md%26commit_hash%3Dc6eb78a44de48735213413b2a28569fdc10466d0)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/20_ownership.md&commit_hash=c6eb78a44de48735213413b2a28569fdc10466d0)

ErgはPythonをホスト言語にした言語であるため、メモリ管理の方法はPythonの処理系に依存しています。
しかし、意味論的にはErgのメモリ管理はPythonのそれとは別物です。顕著な違いは、所有権システムと循環参照の禁止に現れています。

## 所有権

ErgはRustから影響を受けた所有権システムを持っています。
Rustの所有権システムは一般的に難解だと言われていますが、Ergのそれは直感的になるよう簡略化されています。
Ergでは __可変オブジェクト__ に所有権がついており、所有権を失った後はそのオブジェクトを参照できません。

```python
v = [1, 2, 3].into [Int; !3]

push! vec, x =
    vec.push!(x)
    vec

# vの中身([1, 2, 3])の所有権はwに移る
w = push! v, 4
print! v # error: v was moved
print! w # [1, 2, 3, 4]
```

所有権の移動はオブジェクトをサブルーチンに渡したときなどに発生します。
渡した後も所有権をまだ持っていたい場合は、複製(cloning)、凍結(freeze)、または借用(borrowing)をする必要があります。
ただし、後述するように借用はできる場面が限られています。

## 複製

オブジェクトを複製してその所有権を移します。実引数に`.clone`メソッドを適用することで行います。
複製したオブジェクトは複製元のオブジェクトと全く同一になりますが、互いに独立しているので、変更の影響は受けません。

複製はPythonのディープコピーに相当し、同一のオブジェクトをまるごと作り直すので、凍結・借用と比べて一般に計算コスト、メモリコストが高くなります。
オブジェクトを複製する必要があるようなサブルーチンは、「引数を消費する」サブルーチンといいます。

```python
capitalize s: Str! =
    s.capitalize!()
    s

s1 = !"hello"
s2 = capitalize s1.copy()
log s2, s1 # !"HELLO hello"
```

## 凍結

不変オブジェクトは複数の場所から参照できることを利用して、可変オブジェクトを不変オブジェクトに変換します。
これを凍結といいます。凍結は可変リストからイテレータを作るときなどで使われます。
可変リストからは直接イテレータを作ることができないので、不変リストに変換します。
リストを壊したくない場合は、[`.freeze_map`メソッド](./type/18_mut.md)等を使います。

```python
# イテレータが出す値の合計を計算する
sum|T <: Add + HasUnit| i: Iterator T = ...

x = [1, 2, 3].into [Int; !3]
x.push!(4)
i = x.iter()
assert sum(i) == 10
y # この後もyは触れられる
```

## 借用

借用は複製や凍結よりも低コストです。
以下のような単純な場合では、借用を行えます。

```python
peek_str ref(s: Str!) =
    log s

s = !"hello"
peek_str s
```

借用した値は元のオブジェクトに対する __参照__ と呼ばれます。
参照をまた別のサブルーチンに渡す「又貸し」はできますが、借りているだけなので消費することはできません。

```python,compile_fail
steal_str ref(s: Str!) =
    # log関数は引数を借用するだけなので、又貸しできる
    log s
    # discard関数は引数を消費するので、エラー
    discard s # OwnershipError: cannot consume a borrowed value
    # hint: use `clone` method
```

```python,compile_fail
steal_str ref(s: Str!) =
    # これもエラー(=は右辺を消費する)
    x = s # OwnershipError: cannot consume a borrowed value
    x
```

Ergの参照はRustより制約が強いです。参照は言語上第一級のオブジェクトですが、明示的に生成することはできず、`ref`/`ref!`によって実引数の渡し方として指定できるのみです。
これは、参照をリストに詰めたり参照を属性とするクラスを作ったりはできないということを意味します。

とはいえ、このような制約はそもそも参照のない言語では当たり前の仕様であり、そこまで不便となることはありません。

## 循環参照

Ergでは意図せずメモリリークを起こせないように設計されており、メモリーチェッカーが循環参照を検知するとエラーを出します。ほとんどの場合、このエラーは弱参照`Weak`で解消できます。しかし、これでは巡回グラフなどの循環構造を持つオブジェクトを生成できないため、unsafe操作として循環参照を生成できるAPIを実装予定です。

<p align='center'>
    <a href='./19_mutability.md'>Previous</a> | <a href='./21_visibility.md'>Next</a>
</p>

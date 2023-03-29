# 部分型付け

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/16_subtyping.md%26commit_hash%3Db713e6f5cf9570255ccf44d14166cb2a9984f55a)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/16_subtyping.md&commit_hash=b713e6f5cf9570255ccf44d14166cb2a9984f55a)

Ergでは、クラス同士の包含関係は比較演算子`<`, `>`で判定可能です。

```python
Nat < Int
Int < Object
1.._ < Nat
{1, 2} > {1}
{=} > {x = Int}
{I: Int | I >= 1} < {I: Int | I >= 0}
```

`<:`演算子とは別の意味を持つことに注意してください。左辺のクラスが右辺の型のサブタイプであると宣言するもので、コンパイル時にのみ意味を持ちます。

```python
C <: T # T: StructuralType
f|D <: E| ...

assert F < G
```

また、多相型の部分型指定について、例えば`Self(R, O) <: Add(R, O)`などの場合、`Self <: Add`と指定することもできます。

## 構造型、クラスの型関係

構造型は構造的型付けを実現するための型であり、構造が同じならば同じオブジェクトとみなされます。

```python
T = Structural {i = Int}
U = Structural {i = Int}

assert T == U
t: T = {i = 1}
assert t in T
assert t in U
```

対してクラスは記名的型付けを実現するための型であり、型およびインスタンスを構造的に比較することができません。

```python
C = Class {i = Int}
D = Class {i = Int}

assert C == D # TypeError: cannot compare classes
c = C.new {i = 1}
assert c in C
assert not c in D
```

## サブルーチンの部分型付け

サブルーチンの引数、戻り値は、単一のクラスのみを取る。
すなわち、構造型やトレイトを関数の型として直接指定することはできない。
部分型指定を使って「その型のサブタイプである単一のクラス」として指定する必要がある。

```python
# OK
f1 x, y: Int = x + y
# NG
f2 x, y: Add = x + y
# OK
# Aは何らかの具体的なクラス
f3<A <: Add> x, y: A = x + y
```

サブルーチンの型推論もこのルールに従っている。サブルーチン中の変数で型が明示されていないものがあったとき、コンパイラはまずその変数がいずれかのクラスのインスタンスでないかチェックし、そうでない場合はスコープ中のトレイトの中から適合するものを探す。それでも見つからない場合、コンパイルエラーとなる。このエラーは構造型を使用することで解消できるが、無名型を推論するのはプログラマの意図しない結果である可能性があるため、プログラマが明示的に`Structural`で指定する設計となっている。

## クラスのアップキャスト

```python
i: Int
i as (Int or Str)
i as (1..10)
i as {I: Int | I >= 0}

```
<p align='center'>
    <a href='./15_quantified.md'>Previous</a> | <a href='./17_type_casting.md'>Next</a>
</p>

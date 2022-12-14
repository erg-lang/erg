# パッチメソッドの解決

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/compiler/trait_method_resolving.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/compiler/trait_method_resolving.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

`Nat`は0以上の`Int`、つまり`Int`のサブタイプである。
本来`Nat`はPythonのクラス階層には存在しない。Ergはこのパッチのメソッドをどうやって解決するのだろうか?

```python
1.times do:
    log "hello, world"
```

`.times`は`NatImpl`のパッチメソッドである。
`1`は`Int`のインスタンスであるので、まず`Int`のMRO(Method Resolution Order)を辿って探索する。
Ergは`Int`のMROに`Int`, `Object`を持っている。これはPython由来である(Pythonにおいて`int.__mro__ == [int, object]`)。
`.times`メソッドはそのどちらにも存在しない。ここからは、そのサブタイプの探索に入る。

~

整数は明らかにその上位型に実数や複素数、ひいては数全体を持つはずだが、Pythonと互換性をもつレイヤーではその事実は現れない。
だが実際にErgでは`1 in Complex`や`1 in Num`は`True`となる。
`Complex`に至っては、`Int`と継承関係にないクラスであるのに、型として互換性があると判断されている。一体どうなっているのか。

~

あるオブジェクトに対して、その属する型は無数に存在する。
だが実際に考えなくてはならないのはメソッドを持つ型、すなわち名前を持つ型のみである。

Ergコンパイラは、全ての提供メソッドとその実装を持つパッチ・型のハッシュマップを持っている。
このテーブルは型が新たに定義されるたびに更新される。

```python
provided_method_table = {
    ...
    "foo": [Foo],
    ...
    ".times": [Nat, Foo],
    ...
}
```

`.times`メソッドを持つ型は`Nat`, `Foo`である。これらの中から、`{1}`型に適合するものを探す。
適合判定は二種類ある。篩型判定とレコード型判定である。篩型判定から行われる。

## 篩型判定

候補の型が`1`の型`{1}`と互換性があるか確認する。篩型の中で`{1}`と互換性があるのは、`{0, 1}`, `0..9`などである。
`0..1 or 3..4`, `-1..2 and 0..3`などの有限要素の代数演算型は、基底型として宣言すると篩型に正規化される(つまり、`{0, 1, 3, 4}`, `{0, 1, 2}`にする)。
今回の場合、`Nat`は`0.._ == {I: Int | I >= 0}`であるので、`{1}`は`Nat`と互換性がある。

## レコード型判定

候補の型が1のクラスである`Int`と互換性を持つか確認する。
その他、`Int`のパッチである、またその要求属性を`Int`がすべて持つ場合も互換性がある。

~

というわけで、`Nat`が適合した。ただ`Foo`も適合してしまった場合は、`Nat`と`Foo`の包含関係によって判定される。
すなわち、サブタイプのメソッドが選択される。
両者に包含関係がない場合は、コンパイルエラーとなる(これはプログラマーの意図に反したメソッドが実行されないための安全策である)。
エラーを解消させるためには、パッチを明示的に指定する必要がある。

```python
o.method(x) -> P.method(o, x)
```

## 全称パッチのメソッド解決

以下のようなパッチを定義する。

```python
FnType T: Type = Patch T -> T
FnType.type = T
```

`FnType`パッチのもとで以下のようなコードが可能である。これはどのように解決されるのだろうか。

```python
assert (Int -> Int).type == Int
```

まず、`provided_method_table`には`FnType(T)`が以下の形式で登録される。

```python
provided_method_table = {
    ...
    "type": [FnType(T)],
    ...
}
```

`FnType(T)`のパッチする型が適合するかチェックされる。この場合、`FnType(T)`のパッチ型は`Type -> Type`である。
これは`Int -> Int`に適合する。適合したら、単相化を行って置換する(`T -> T`と`Int -> Int`のdiffを取る。`{T => Int}`)。

```python
assert FnType(Int).type == Int
```

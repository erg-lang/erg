# 型消去

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/type/advanced/erasure.md%26commit_hash%3Dc6eb78a44de48735213413b2a28569fdc10466d0)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/type/advanced/erasure.md&commit_hash=c6eb78a44de48735213413b2a28569fdc10466d0)

型消去とは、型引数に`_`を指定し、その情報をあえて捨てることです。型消去は多相型を持つ言語の多くが併せて持つ機能ですが、Ergの文法に即して言えば型引数消去といった方が正確でしょう。

もっともよく見られる型消去された型の例は`[T, _]`でしょうリストはコンパイル時にその長さが分からない場合もあります。例えば、コマンドライン引数を指す`sys.argv`は`[Str, _]`型です。コマンドライン引数の長さをErgのコンパイラは知りようがないため、長さに関する情報は諦めなくてはならないのです。
しかし、型消去された型は、されていない型のスーパータイプになる(e.g. `[T; N] < [T; _]`)ため、より多くのオブジェクトを受け取れるようになります。
`[T; N]`型のオブジェクトはもちろん`[T; _]`型のメソッドを使用できますが、使用後`n`の情報は消去されます。長さが変わってしまっているかもしれないからです。長さが変わらないならばシグネチャで示さなくてはなりません。

```python
# リストの長さが変わらないことが保証される関数(sortなど)
f: [T; N] -> [T; N]
# 長さが保障されない関数(filterなど)
g: [T; n] -> [T; _]
```

型指定自体で`_`を使うとその型は`Object`までアップキャストされます。
型でない型引数(Int, Bool型など)の場合、`_`としたパラメータは未定義になります。

```python
i: _ # i: Object
[_; _] == [Object; _] == List
```

型消去は型指定の省略とは違います。一度型引数情報を消去してしまうと、再びアサーションしなければ情報は戻りません。

```python
implicit = (1..5).iter().map(i -> i * 2).to_arr()
explicit = (1..5).iter().map(i -> i * 2).into(List(Nat))
```

Rustでは以下のコードに対応します。

```rust
let partial = (1..6).iter().map(|i| i * 2).collect::<Vec<_>>();
```

Ergでは型の部分省略はできず、代わりに高階カインド多相を使用します。

```python
# collectはカインドを受け取る高階カインドのメソッド
hk = (1..5).iter().map(i -> i * 2).collect(List)
hk: List(Int)
```

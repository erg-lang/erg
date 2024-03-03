# プロシージャ

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/08_procedure.md%26commit_hash%3D637109aa8b3826b78df334ef6508131cff575623)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/08_procedure.md&commit_hash=637109aa8b3826b78df334ef6508131cff575623)

プロシージャは[副作用](./07_side_effect.md)を許容する関数を意味します。
基本的な定義や利用方法は[関数](./04_function.md)を参照してください。
関数名に対して`!`をつけることで定義することができます。

```python
proc!(x: Int!, y: Int!) =
    for! 0..x, i =>
        for 0..y, j =>
            print! i, j
```

プロシージャは可変オブジェクトを取り扱う際に必要となります。
ですが、可変オブジェクトを引数に持つときやプロシージャの定義するだけの場合はプロシージャであるとは限りません。

```python
peek_str s: Str! = log s

make_proc(x!: (Int => Int)): (Int => Int) = y => x! y
p! = make_proc(x => x)
print! p! 1 # 1
```

またプロシージャと関数は`proc :> func`の関係にあります。
そのため、プロシージャ内で関数ブロックを定義することもできます。
しかし、逆はできないので注意をしてください。

```python
proc!(x: Int!) = y -> log x, y # OK
func(x: Int) = y => print! x, y # NG
```

## バインド

プロシージャはスコープ外の可変変数を操作することができます。

```python
x = !0
proc!() =
    x.inc!()

proc!()
assert x == 1
```

このとき、`proc!`は以下のような型を持ちます。

```python
proc!: {|x: Int!|}() => ()
```

`{|x: Int!|}`の部分はバインド列と呼ばれ、そのプロシージャが操作する変数とその型を表します。
バインド列は自動で導出されるため、明示的に書く必要はありません。

注意として、通常のプロシージャは予め決められた外部変数のみを操作することができます。これはつまり、引数に渡された変数を書き換えることはできないということです。
そのようなことがしたい場合は、プロシージャルメソッドを使う必要があります。プロシージャルメソッドは、`self`を書き換えることができます。

```python
C! N = Class {arr = [Int; N]!}
C!.
    new() = Self!(0) {arr = ![]}
C!(N).
    # push!: {|self: C!(N) ~> C!(N+1)|}(self: RefMut(C!(N)), x: Int) => NoneType
    push! ref! self, x = self.arr.push!(x)
    # pop!: {|self: C!(N) ~> C!(N-1)|}(self: RefMut(C!(N))) => Int
    pop! ref! self = self.arr.pop!()

c = C!.new()
c.push!(1)
assert c.pop!() == 1
```

<p align='center'>
    <a href='./07_side_effect.md'>Previous</a> | <a href='./09_builtin_procs.md'>Next</a>
</p>

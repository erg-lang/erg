# プロシージャ

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/08_procedure.md%26commit_hash%3D96b113c47ec6ca7ad91a6b486d55758de00d557d)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/08_procedure.md&commit_hash=96b113c47ec6ca7ad91a6b486d55758de00d557d)

プロシージャは[副作用](/07_side_effect.md)を許容する関数を意味します。
基本的な定義や利用方法は[関数](/04_function.md)を参照してください。
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

<p align='center'>
    <a href='./07_side_effect.md'>Previous</a> | <a href='./09_builtin_procs.md'>Next</a>
</p>

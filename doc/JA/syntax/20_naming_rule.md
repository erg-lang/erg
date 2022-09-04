# 命名規則

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/20_naming_rule.md%26commit_hash%3Deccd113c1512076c367fb87ea73406f91ff83ba7)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/20_naming_rule.md&commit_hash=eccd113c1512076c367fb87ea73406f91ff83ba7)

変数を定数式として使いたい場合は、必ず大文字で始めます。二文字以降は小文字でもよいです。

```erg
i: Option Type = Int
match i:
    t: Type -> log "type"
    None -> log "None"
```

副作用のあるオブジェクトは、必ず`!`で終わります。プロシージャとプロシージャルメソッド、そして可変型です。
ただし、`Proc`型自体は可変型ではありません。

```erg
# Callable == Func or Proc
c: Callable = print!
match c:
    p! -> log "proc" # 自明なので`: Proc`を省略可
    f -> log "func"
```

属性を外部に公開したい場合は、初めに`.`をつけて定義します。`.`を初めにつけなかった場合は非公開になります。混乱を避けるため同一のスコープ内で共存はできません。

```erg
o = {x = 1; .x = 2} # SyntaxError: private and public variables with the same name cannot coexist
```

## リテラル識別子(Literal Identifiers)

以上の規則は、文字列をシングルクォート('')で囲むと回避できます。すなわち、プロシージャルオブジェクトも`!`をつけずに代入することができます。ただしこの場合、値が定数式でも定数とはみなされません。
このようにシングルクォートで囲まれた文字列による識別子をリテラル識別子といいます。
これは、Pythonなど他言語のAPI(FFI)を呼び出す際に使います。

```erg
bar! = pyimport("foo").'bar'
```

Ergでも有効な識別子の場合は、''で囲む必要はありません。

さらに、リテラル識別子中では記号も空白も入れることができるため、通常は識別子として使えない文字列を識別子として使うことができます。

```erg
'∂/∂t' y
'test 1: pass x to y'()
```

<p align='center'>
    <a href='./19_visibility.md'>Previous</a> | <a href='./21_lambda.md'>Next</a>
</p>

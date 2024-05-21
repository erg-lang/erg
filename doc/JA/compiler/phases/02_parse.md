# Parsing (構文解析)

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/compiler/phases/02_parse.md%26commit_hash%3D19bab4ae63af9415da20ebd7499c668144da5ea6)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/compiler/phases/02_prarse.md&commit_hash=19bab4ae63af9415da20ebd7499c668144da5ea6)

構文解析を行うのは`erg_parser/parse.rs`に定義される`Parser`である。これも使い捨ての構造体であり、主に`ParserRunner`でラップして使う。
`Parser`は再帰下降構文解析を行う。スタックオーバーフローを避けるため、デフォルトのスタックが小さいWindowsでは、手動でスタックサイズが指定された別スレッド上で実行される。

Ergの文法の特徴的な点は、case-sensitiveであること、また最悪の場合いくら先読みしても文法が確定しないことである。

例えば、以下の構文を考える。

```python
a, b, c, d, e, (...)
```

(...)の中で=が現れればこれはタプルの分割代入と判明する。現れなければ単なるタプルである。
しかし、どちらであるかを決定するのに必要なトークンの数は上限がない。

そこで`Parser`は上のような場合、まずタプルであると決め打ちして解析を進める。
改行が来る前に=または->, =>が来たら、これは分割代入であると判明し、今まで解析したタプルを左辺値に変換する。
その他のパターン、関数定義もこれと同様の方法で解析される。
このようなことが可能なのは、すべての左辺値に対して構文的に双対となる右辺値が存在するからである(しかし、すべての右辺値に対して双対となる左辺値があるわけでない)。

# Lexing (字句解析)

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/compiler/phases/01_lex.md%26commit_hash%3D19bab4ae63af9415da20ebd7499c668144da5ea6)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/compiler/phases/01_lex.md&commit_hash=19bab4ae63af9415da20ebd7499c668144da5ea6)

字句解析を行うのは`erg_parser/lex.rs`に定義される`Lexer`である。
これはイテレータとして実装されており、`Token`という構造体を返す。
`Token`はErgの字句を表す構造体で、種別として`TokenKind`, ソースコード上の位置情報、そして文字列表現を持つ。
`Token`は`Locational`トレイトを実装する最小の構造体である。`Locational`トレイトは`Location`という列挙体を返す`loc()`メソッドを持つ。
これはソースコードの位置を表す。

イテレータであることからも分かるように、Lexerは使い捨ての構造体である。
これを連続して使えるようにラップしたのが`LexerRunner`である。この構造体は`Runnable`トレイトを実装しており、コマンドラインオプションを渡して実行したり、REPLとしても使ったりすることができる。

ErgのLexerの特徴的な点は、インデントを字句として扱うことである。これはPython等のIndent-sensitive言語の字句解析と同じである。

ErgのLexerはインデント/Dedentの数が合うかチェックするが、文法的に正しい使い方をしているかはチェックしない。
例えば、字句解析の時点では以下のコードはエラーにならない。

```python
i = 1
    j = 2
k = 3
```

これがエラーになるのは`Parser`の構文解析時である。

# Lexing

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/compiler/phases/01_lex.md%26commit_hash%3D85d800653fb1ab12805dc82f7c3b501de8ba63b6)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/compiler/phases/01_lex.md&commit_hash=85d800653fb1ab12805dc82f7c3b501de8ba63b6)

The `Lexer` defined in `erg_parser/lex.rs` performs lexical analysis.
It is implemented as an iterator and returns a structure called `Token`.
`Token` is a structure representing a lexeme in Erg, and it has a type called `TokenKind`, location information in the source code, and a string representation.
`Token` is the smallest structure that implements the `Locational` trait. The `Locational` trait has a method called `loc()` that returns an enumeration called `Location`.
This represents the position in the source code.

As can be inferred from being an iterator, the Lexer is a disposable structure.
`LexerRunner` wraps it to make it usable consecutively. This structure implements the `Runnable` trait, and can be executed by passing command-line options or used as a REPL.

A distinctive feature of Erg's Lexer is that it treats indents as lexemes, similar to the lexical analysis of indent-sensitive languages like Python.

Erg's Lexer checks if the number of indents/dedents matches, but it does not check if they are used correctly grammatically.
For example, the following code does not result in an error at the lexical analysis stage.

```python
i = 1
    j = 2
k = 3
```

This will result in a syntax error at the parsing stage.

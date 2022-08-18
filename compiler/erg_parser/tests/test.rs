use std::iter::Iterator;

use erg_common::config::Input;

// use erg_compiler::parser;

use erg_parser::error::ParseResult;
use erg_parser::lex::Lexer;
use erg_parser::token::*;
use TokenKind::*;

const FILE1: &str = "tests/test1_basic_syntax.er";

#[test]
fn test_lexer() -> ParseResult<()> {
    let mut lexer = Lexer::new(Input::File(FILE1.into()));
    let newline = "\n";
    let /*mut*/ token_array = vec![
            (Symbol, "_a"),
            (Equal, "="),
            (IntLit, "1234"),
            (Plus, "+"),
            (RatioLit, "1113.0"),
            (Plus, "+"),
            (RatioLit, "0.30102"),
            // (Symbol, "a"),
            (Newline, newline),
            (Symbol, "a"),
            (Comma, ","),
            (UBar, "_"),
            (Comma, ","),
            (Spread, "..."),
            (Symbol, "b"),
            (Equal, "="),
            (Symbol, "five_elem_tuple"),
            (Newline, newline),
            (Symbol, "if!"),
            (Symbol, "True"),
            (Comma, ","),
            (Symbol, "do!"),
            (Newline, newline),
            (Indent, "    "),
            (Symbol, "print!"),
            // (LParen, "("),
            (StrLit, "\\\\hello, world\\\""),
            // (RParen, ")"),
            (Newline, newline),
            (IntLit, "10"),
            (Dot, "."),
            (Symbol, "times!"),
            // (LParen, "("),
            // (RParen, ")"),
            (Symbol, "do!"),
            (Newline, newline),
            (Indent, "    "),
            (Symbol, "if!"),
            (Symbol, "True"),
            (Comma, ","),
            (Symbol, "do!"),
            (Newline, newline),
            (Indent, "    "),
            (Symbol, "print!"),
            (StrLit, ""),
            (Newline, newline),
            // (Comment, " illegal indent"),
            // (Illegal, "DEDENT"),
            // (Symbol, "do_nothing"),
            (Dedent, ""),
            (Newline, newline),
            (Newline, newline),
            (Symbol, "Hello"),
            (Equal, "="),
            (Symbol, "S2c"),
            // (LParen, "("),
            (StrLit, "hello"),
            // (RParen, ")"),
            (Newline, newline),
            (Dedent, ""),
            (Dedent, ""),
            (Symbol, "aあ아"),
            (Equal, "="),
            (Newline, newline),
            (Indent, "    "),
            (Newline, newline),
            (StrLit, "aaa"),
            (Newline, newline),
            (Dedent, ""),
            (Symbol, "x"),
            (Semi, ";"),
            (Symbol, "x"),
            (Semi, ";"),
            (Semi, ";"),
            (Symbol, "x"),
            (Semi, ";"),
            (Newline, newline),
            (IntLit, "10"),
            (Closed, ".."),
            (Symbol, "twelve"),
            (Semi, ";"),
            (Newline, newline),
            (EOF, "EOF"),
        ];

    let mut tok: Token;
    for i in token_array.into_iter() {
        tok = lexer.next().unwrap().unwrap();
        assert_eq!(tok, Token::from_str(i.0, i.1));
        println!("{tok}");
    }
    Ok(())
}

#[test]
fn tesop_te_prec() {
    assert_eq!(Mod.precedence(), Some(160));
    assert_eq!(LParen.precedence(), Some(0));
    assert_eq!(Illegal.precedence(), None);
}

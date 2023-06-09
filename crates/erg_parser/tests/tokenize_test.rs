use std::iter::Iterator;

use erg_common::io::Input;

// use erg_compiler::parser;

use erg_parser::error::ParseResult;
use erg_parser::lex::Lexer;
use erg_parser::token::*;
use TokenKind::*;

const FILE1: &str = "tests/test1_basic_syntax.er";
const FILE2: &str = "tests/test2_advanced_syntax.er";
const FILE3: &str = "tests/test3_literal_syntax.er";
const FILE4: &str = "tests/multi_line_str_literal.er";
const FILE5: &str = "tests/for.er";

#[test]
fn test_lexer_for_basic() -> ParseResult<()> {
    let mut lexer = Lexer::new(Input::file(FILE1.into()));
    let newline = "\n";
    let token_array = vec![
        (Newline, newline),
        (Newline, newline),
        (Symbol, "_a"),
        (Assign, "="),
        (NatLit, "1_234"),
        (Plus, "+"),
        (RatioLit, "1113."),
        (Star, "*"),
        (RatioLit, "3_000.2e-4"),
        (Pow, "**"),
        (NatLit, "0003"),
        (Star, "*"),
        (RatioLit, ".4"),
        (Newline, newline),
        (Symbol, "a"),
        (Comma, ","),
        (UBar, "_"),
        (Comma, ","),
        (Symbol, "b"),
        (Assign, "="),
        (Symbol, "tuple"),
        (Newline, newline),
        (Symbol, "f"),
        (Symbol, "x"),
        (Comma, ","),
        (Symbol, "y"),
        (Assign, "="),
        (Newline, newline),
        (Indent, "    "),
        (Symbol, "x"),
        (Plus, "+"),
        (Symbol, "y"),
        (Newline, newline),
        (Dedent, ""),
        (Symbol, "if!"),
        (BoolLit, "True"),
        (Comma, ","),
        (Symbol, "do!"),
        (Colon, ":"),
        (Newline, newline),
        (Indent, "    "),
        (Symbol, "print!"),
        (StrLit, "\"\"\\hello, world\\\"\""),
        (Newline, newline),
        (Newline, newline),
        (NatLit, "10"),
        (Dot, "."),
        (Symbol, "times!"),
        (Symbol, "do!"),
        (Colon, ":"),
        (Newline, newline),
        (Indent, "    "),
        (Symbol, "if!"),
        (Symbol, "x"),
        (Dot, "."),
        (Symbol, "y"),
        (Dot, "."),
        (Symbol, "z"),
        (Comma, ","),
        (Symbol, "do!"),
        (Colon, ":"),
        (Newline, newline),
        (Indent, "    "),
        (Symbol, "print!"),
        (StrLit, "\"\""),
        (Newline, newline),
        (Dedent, ""),
        (Symbol, "Hello"),
        (Assign, "="),
        (Symbol, "S2c"),
        (StrLit, "\"hello\""),
        (Newline, newline),
        (NoneLit, "None"),
        (Newline, newline),
        (Dedent, ""),
        (Dedent, ""),
        (Symbol, "aあ아"),
        (Assign, "="),
        (Newline, newline),
        (Indent, "    "),
        (Newline, newline),
        (StrLit, "\"aaa\""),
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
        (NatLit, "10"),
        (Closed, ".."),
        (Symbol, "twelve"),
        (Semi, ";"),
        (Newline, newline),
        (EOF, "\0"),
    ];
    let mut tok: Token;
    for (id, i) in token_array.into_iter().enumerate() {
        print!("{id:>03}: ");
        tok = lexer.next().unwrap().unwrap();
        assert_eq!(tok, Token::from_str(i.0, i.1));
        println!("{tok}");
    }
    Ok(())
}

#[test]
fn test_lexer_for_advanced() -> ParseResult<()> {
    let mut lexer = Lexer::new(Input::file(FILE2.into()));
    let newline = "\n";
    let token_array = vec![
        (Newline, newline),
        (Newline, newline),
        (Newline, newline),
        (Newline, newline),
        (Symbol, "fib"),
        (NatLit, "0"),
        (Assign, "="),
        (NatLit, "0"),
        (Newline, newline),
        (Symbol, "fib"),
        (NatLit, "1"),
        (Assign, "="),
        (NatLit, "1"),
        (Newline, newline),
        (Symbol, "fib"),
        (LParen, "("),
        (Symbol, "n"),
        (Colon, ":"),
        (Symbol, "Nat"),
        (RParen, ")"),
        (Colon, ":"),
        (Symbol, "Nat"),
        (Assign, "="),
        (Symbol, "fib"),
        (LParen, "("),
        (Symbol, "n"),
        (Minus, "-"),
        (NatLit, "1"),
        (RParen, ")"),
        (Plus, "+"),
        (Symbol, "fib"),
        (LParen, "("),
        (Symbol, "n"),
        (Minus, "-"),
        (NatLit, "2"),
        (RParen, ")"),
        (Newline, newline),
        (Newline, newline),
        (Newline, newline),
        (Symbol, "t"),
        (Assign, "="),
        (Symbol, "if"),
        (BoolLit, "True"),
        (Colon, ":"),
        (Newline, newline),
        (Indent, "    "),
        (Symbol, "then"),
        (Walrus, ":="),
        (NatLit, "1"),
        (Newline, newline),
        (Symbol, "else"),
        (Walrus, ":="),
        (NatLit, "2"),
        (Newline, newline),
        (Dedent, ""),
        (Symbol, "assert"),
        (Symbol, "t"),
        (DblEq, "=="),
        (NatLit, "1"),
        (Newline, newline),
        (Newline, newline),
        (Newline, newline),
        (Symbol, "math"),
        (Assign, "="),
        (Symbol, "import"),
        (StrLit, "\"math\""),
        (Newline, newline),
        (Newline, newline),
        (LBrace, "{"),
        (Symbol, "pi"),
        (Semi, ";"),
        (RBrace, "}"),
        (Assign, "="),
        (Symbol, "import"),
        (StrLit, "\"math\""),
        (Newline, newline),
        (EOF, "\0"),
    ];
    let mut tok: Token;
    for (id, i) in token_array.into_iter().enumerate() {
        print!("{id:>03}: ");
        tok = lexer.next().unwrap().unwrap();
        assert_eq!(tok, Token::from_str(i.0, i.1));
        println!("{tok}");
    }
    Ok(())
}

#[test]
fn test_lexer_for_literals() -> ParseResult<()> {
    let mut lexer = Lexer::new(Input::file(FILE3.into()));
    let newline = "\n";
    let token_array = vec![
        (Newline, newline),
        (Newline, newline),
        (NatLit, "0"),
        (Comma, ","),
        (NatLit, "1"),
        (Comma, ","),
        (NatLit, "2"),
        (Comma, ","),
        (NatLit, "100_000"),
        (Newline, newline),
        (Newline, newline),
        (Newline, newline),
        (IntLit, "-1"),
        (Comma, ","),
        (IntLit, "-10"),
        (Comma, ","),
        (IntLit, "-100_000"),
        (Newline, newline),
        (Newline, newline),
        (Newline, newline),
        (RatioLit, "0.00"),
        (Comma, ","),
        (RatioLit, "-0.0"),
        (Comma, ","),
        (RatioLit, ".1"),
        (Comma, ","),
        (RatioLit, "400."),
        (Newline, newline),
        (Newline, newline),
        (Newline, newline),
        (StrLit, "\"\""),
        (Comma, ","),
        (StrLit, "\"a\""),
        (Comma, ","),
        (StrLit, "\"こんにちは\""),
        (Comma, ","),
        (StrLit, "\"\"\\\""),
        (Comma, ","),
        (StrLit, "\"\"\'\\\0\r\n    \""),
        (Newline, newline),
        (Newline, newline),
        (Newline, newline),
        (BoolLit, "True"),
        (Comma, ","),
        (BoolLit, "False"),
        (Newline, newline),
        (Newline, newline),
        (Newline, newline),
        (NoneLit, "None"),
        (Newline, newline),
        (Newline, newline),
        (Newline, newline),
        (EllipsisLit, "..."),
        (Newline, newline),
        (Newline, newline),
        (Newline, newline),
        (InfLit, "Inf"),
        (Comma, ","),
        (PreMinus, "-"),
        (InfLit, "Inf"),
        (Newline, newline),
        (Newline, newline),
        (Newline, newline),
        (Symbol, "NotImplemented"),
        (Newline, newline),
        (Newline, newline),
        (Newline, newline),
        (Newline, newline),
        (EOF, "\0"),
    ];
    let mut tok: Token;
    for (id, i) in token_array.into_iter().enumerate() {
        print!("{id:>03}: ");
        tok = lexer.next().unwrap().unwrap();
        assert_eq!(tok, Token::from_str(i.0, i.1));
        println!("{tok}");
    }
    Ok(())
}

#[test]
fn test_lexer_for_multi_line_str_literal() -> ParseResult<()> {
    let mut lexer = Lexer::new(Input::file(FILE4.into()));
    let newline = "\n";
    let token_array = [
        (Newline, newline),
        (Newline, newline),
        (Symbol, "single_a"),
        (Assign, "="),
        (StrLit, "\"line break\naaa\nbbb\nccc\nline break\""),
        (Newline, newline),
        (Symbol, "print!"),
        (Symbol, "single_a"),
        (Newline, newline),
        (Newline, newline),
        (Symbol, "multi_line_a"),
        (Assign, "="),
        (
            StrLit,
            "\"\"\"line break
aaa
bbb
ccc
line break\"\"\"",
        ),
        (Newline, newline),
        (Symbol, "print!"),
        (Symbol, "multi_line_a"),
        (Newline, newline),
        (Newline, newline),
        (Symbol, "single_b"),
        (Assign, "="),
        (
            StrLit,
            "\"ignore line break    Hello,\n    Worldignored line break\"",
        ),
        (Newline, newline),
        (Symbol, "print!"),
        (Symbol, "single_b"),
        (Newline, newline),
        (Newline, newline),
        (Symbol, "multi_line_b"),
        (Assign, "="),
        (
            StrLit,
            "\"\"\"ignore line break    Hello,
    Worldignored line break\"\"\"",
        ),
        (Newline, newline),
        (Symbol, "print!"),
        (Symbol, "multi_line_b"),
        (Newline, newline),
        (Newline, newline),
        (Symbol, "complex_string"),
        (Assign, "="),
        (StrLit, "\"\"\"\\    \0\r\n\\\"\"\""),
        (Newline, newline),
        (Symbol, "print!"),
        (Symbol, "complex_string"),
        (Newline, newline),
        (Newline, newline),
        (Symbol, "quotation_mark"),
        (Assign, "="),
        (StrLit, "\"\"\"\"\"\"\"\"\""),
        (Newline, newline),
        (Symbol, "print!"),
        (Symbol, "quotation_mark"),
        (Newline, newline),
        (Newline, newline),
        (Symbol, "quotation_marks"),
        (Assign, "="),
        (
            StrLit,
            "\"\"\"\"
\"\"
\"\"\"
\"\"\"\"\"
\"\"\"
\"\"\"\"\"\"",
        ),
        (Newline, newline),
        (Symbol, "print!"),
        (Symbol, "quotation_marks"),
        (Newline, newline),
        (EOF, "\0"),
    ];
    let mut tok: Token;
    for (id, i) in token_array.into_iter().enumerate() {
        print!("{id:>03}: ");
        tok = lexer.next().unwrap().unwrap();
        assert_eq!(tok, Token::from_str(i.0, i.1));
        println!("{tok}");
    }
    Ok(())
}

#[test]
fn for_loop() -> ParseResult<()> {
    let mut lexer = Lexer::new(Input::file(FILE5.into()));
    let newline = "\n";
    let token_array = [
        (Symbol, "for!"),
        (NatLit, "0"),
        (Closed, ".."),
        (NatLit, "1"),
        (Comma, ","),
        (Symbol, "i"),
        (ProcArrow, "=>"),
        (Newline, newline),
        (Indent, "    "),
        (Symbol, "for!"),
        (NatLit, "0"),
        (Closed, ".."),
        (NatLit, "1"),
        (Comma, ","),
        (Symbol, "j"),
        (ProcArrow, "=>"),
        (Newline, newline),
        (Indent, "    "),
        (Symbol, "for!"),
        (NatLit, "0"),
        (Closed, ".."),
        (NatLit, "1"),
        (Comma, ","),
        (Symbol, "k"),
        (ProcArrow, "=>"),
        (Newline, newline),
        (Newline, newline),
        (Indent, "    "),
        (Symbol, "print!"),
        (StrLit, "\"hi\""),
        (Newline, newline),
        (Dedent, ""),
        (Dedent, ""),
        (Dedent, ""),
        (EOF, "\0"),
    ];
    let mut tok: Token;
    for (id, i) in token_array.into_iter().enumerate() {
        print!("{id:>03}: ");
        tok = lexer.next().unwrap().unwrap();
        assert_eq!(tok, Token::from_str(i.0, i.1));
        println!("{tok}");
    }
    Ok(())
}

#[test]
fn tesop_te_prec() {
    assert_eq!(Mod.precedence(), Some(170));
    assert_eq!(LParen.precedence(), Some(0));
    assert_eq!(Illegal.precedence(), None);
}

extern crate common;
extern crate parser;

mod tests {
    use std::iter::Iterator;

    use erg_common::config::{ErgConfig, Input};
    use erg_common::error::MultiErrorFmt;
    use erg_common::traits::Runnable;

    // use erg_compiler::parser;

    use erg_parser::error::*;
    use erg_parser::lex::Lexer;
    use erg_parser::token::*;
    use erg_parser::ParserRunner;
    use TokenKind::*;

    const FILE1: &str = "src/compiler/parser/tests/test1_basic_syntax.er";

    #[test]
    fn test_lexer() -> ParseResult<()> {
        let mut cfg = ErgConfig::default();
        cfg.set_input(Input::File(FILE1));
        let mut lexer = Lexer::new(cfg);
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
            (Let, "="),
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
            (Let, "="),
            (Symbol, "S2c"),
            // (LParen, "("),
            (StrLit, "hello"),
            // (RParen, ")"),
            (Newline, newline),
            (Dedent, ""),
            (Dedent, ""),
            (Symbol, "aあ아"),
            (Let, "="),
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
            (Range, ".."),
            (Symbol, "twelve"),
            (Semi, ";"),
            (Newline, newline),
            (EOF, "EOF"),
        ];

        let mut tok: Token;
        for i in token_array.into_iter() {
            tok = lexer.next().unwrap().unwrap();
            assert_eq!(tok, Token::without_loc(i.0, i.1));
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

    #[test]
    fn test_parser1() -> Result<(), ParseErrors> {
        let mut cfg = ErgConfig::default();
        cfg.input = Input::File(FILE1);
        let lexer = Lexer::new(cfg);
        let mut parser = ParserRunner::new(&cfg);
        match parser.parse(lexer.lex()?) {
            Ok(module) => {
                println!("{module}");
                Ok(())
            }
            Err(e) => {
                e.fmt_all_stderr();
                Err(e)
            }
        }
    }
}

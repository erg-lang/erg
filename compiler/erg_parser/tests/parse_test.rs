use erg_common::config::{ErgConfig, Input};
use erg_common::error::MultiErrorDisplay;
use erg_common::traits::Runnable;

use erg_parser::error::ParserRunnerErrors;
use erg_parser::lex::Lexer;
use erg_parser::ParserRunner;

#[test]
fn parse_dependent() -> Result<(), ParserRunnerErrors> {
    parse_test_from_code("tests/dependent.er")
}

#[test]
fn parse_fib() -> Result<(), ParserRunnerErrors> {
    parse_test_from_code("tests/fib.er")
}

#[test]
fn parse_hello_world() -> Result<(), ParserRunnerErrors> {
    parse_test_from_code("tests/hello_world.er")
}

#[test]
fn parse_simple_if() -> Result<(), ParserRunnerErrors> {
    parse_test_from_code("tests/simple_if.er")
}

#[test]
fn parse_stack() -> Result<(), ParserRunnerErrors> {
    parse_test_from_code("tests/stack.er")
}

#[test]
fn parse_test1_basic_syntax() -> Result<(), ParserRunnerErrors> {
    parse_test_from_code("tests/test1_basic_syntax.er")
}

#[test]
fn parse_test2_advanced_syntax() -> Result<(), ParserRunnerErrors> {
    parse_test_from_code("tests/test2_advanced_syntax.er")
}

fn parse_test_from_code(file_path: &'static str) -> Result<(), ParserRunnerErrors> {
    let input = Input::File(file_path.into());
    let cfg = ErgConfig::new("exec", 1, false, None, 100, input.clone(), "<module>", 2);
    let lexer = Lexer::new(input.clone());
    let mut parser = ParserRunner::new(cfg);
    match parser.parse_token_stream(
        lexer
            .lex()
            .map_err(|errs| ParserRunnerErrors::convert(&input, errs))?,
    ) {
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

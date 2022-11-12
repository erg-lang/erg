use erg_common::config::{ErgConfig, Input};
use erg_common::error::MultiErrorDisplay;
use erg_common::style::THEME;
use erg_common::traits::Runnable;

use erg_parser::error::ParserRunnerErrors;
use erg_parser::lex::Lexer;
use erg_parser::ParserRunner;

#[test]
fn parse_str_literal() -> Result<(), ParserRunnerErrors> {
    expect_failure("tests/failed_str_lit.er")
}

#[test]
fn parse_dependent() -> Result<(), ParserRunnerErrors> {
    expect_success("tests/dependent.er")
}

#[test]
fn parse_fib() -> Result<(), ParserRunnerErrors> {
    expect_success("tests/fib.er")
}

#[test]
fn parse_hello_world() -> Result<(), ParserRunnerErrors> {
    expect_success("tests/hello_world.er")
}

#[test]
fn parse_simple_if() -> Result<(), ParserRunnerErrors> {
    expect_success("tests/simple_if.er")
}

#[test]
fn parse_stack() -> Result<(), ParserRunnerErrors> {
    expect_failure("tests/stack.er")
}

#[test]
fn parse_test1_basic_syntax() -> Result<(), ParserRunnerErrors> {
    expect_success("tests/test1_basic_syntax.er")
}

#[test]
fn parse_test2_advanced_syntax() -> Result<(), ParserRunnerErrors> {
    expect_success("tests/test2_advanced_syntax.er")
}

fn parse_test_from_code(file_path: &'static str) -> Result<(), ParserRunnerErrors> {
    let input = Input::File(file_path.into());
    let cfg = ErgConfig {
        input: input.clone(),
        py_server_timeout: 100,
        ..ErgConfig::default()
    };
    let lexer = Lexer::new(input.clone());
    let mut parser = ParserRunner::new(cfg);
    match parser.parse_token_stream(
        lexer
            .lex()
            .map_err(|errs| ParserRunnerErrors::convert(&input, errs, THEME))?,
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

fn expect_success(file_path: &'static str) -> Result<(), ParserRunnerErrors> {
    match parse_test_from_code(file_path) {
        Ok(_) => Ok(()),
        Err(e) => {
            e.fmt_all_stderr();
            Err(e)
        }
    }
}

fn expect_failure(file_path: &'static str) -> Result<(), ParserRunnerErrors> {
    match parse_test_from_code(file_path) {
        Ok(_) => Err(ParserRunnerErrors::empty()),
        Err(_) => Ok(()),
    }
}

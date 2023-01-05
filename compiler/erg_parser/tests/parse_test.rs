use erg_common::config::{ErgConfig, Input};
use erg_common::error::MultiErrorDisplay;
use erg_common::spawn::exec_new_thread;
use erg_common::traits::{Runnable, Stream};

use erg_parser::error::ParserRunnerErrors;
use erg_parser::lex::Lexer;
use erg_parser::ParserRunner;

#[test]
fn parse_dependent() -> Result<(), ()> {
    expect_success("tests/dependent.er")
}

#[test]
fn parse_fib() -> Result<(), ()> {
    expect_success("tests/fib.er")
}

#[test]
fn parse_hello_world() -> Result<(), ()> {
    expect_success("tests/hello_world.er")
}

#[test]
fn parse_simple_if() -> Result<(), ()> {
    expect_success("tests/simple_if.er")
}

#[test]
fn parse_stream() -> Result<(), ()> {
    expect_success("tests/stream.er")
}

#[test]
fn parse_test1_basic_syntax() -> Result<(), ()> {
    expect_success("tests/test1_basic_syntax.er")
}

#[test]
fn parse_test2_advanced_syntax() -> Result<(), ()> {
    expect_success("tests/test2_advanced_syntax.er")
}

#[test]
fn parse_stack() -> Result<(), ()> {
    expect_failure("tests/stack.er", 2)
}

#[test]
fn parse_str_literal() -> Result<(), ()> {
    expect_failure("tests/failed_str_lit.er", 2)
}

#[test]
fn exec_invalid_chunk_prs_err() -> Result<(), ()> {
    expect_failure("tests/invalid_chunk.er", 62)
}

fn _parse_test_from_code(file_path: &'static str) -> Result<(), ParserRunnerErrors> {
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

fn parse_test_from_code(file_path: &'static str) -> Result<(), ParserRunnerErrors> {
    exec_new_thread(move || _parse_test_from_code(file_path))
}

fn expect_success(file_path: &'static str) -> Result<(), ()> {
    match parse_test_from_code(file_path) {
        Ok(_) => Ok(()),
        Err(_) => Err(()),
    }
}

fn expect_failure(file_path: &'static str, errs_len: usize) -> Result<(), ()> {
    match parse_test_from_code(file_path) {
        Ok(_) => Err(()),
        Err(errs) => {
            errs.fmt_all_stderr();
            if errs.len() == errs_len {
                Ok(())
            } else {
                println!("err: error length is not {errs_len} but {}", errs.len());
                Err(())
            }
        }
    }
}

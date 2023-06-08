use erg_common::config::{ErgConfig, Input};
use erg_common::consts::DEBUG_MODE;
use erg_common::error::MultiErrorDisplay;
use erg_common::spawn::exec_new_thread;
use erg_common::traits::{Runnable, Stream};

use erg_parser::error::{ErrorArtifact, ParseWarnings, ParserRunnerErrors};
use erg_parser::lex::Lexer;
use erg_parser::ParserRunner;

#[test]
fn parse_args() -> Result<(), ()> {
    expect_success("tests/args.er", 0)
}

#[test]
fn parse_containers() -> Result<(), ()> {
    expect_success("tests/containers.er", 0)
}

#[test]
fn parse_dependent() -> Result<(), ()> {
    expect_success("tests/dependent.er", 0)
}

#[test]
fn parse_fib() -> Result<(), ()> {
    expect_success("tests/fib.er", 0)
}

#[test]
fn parse_hello_world() -> Result<(), ()> {
    expect_success("tests/hello_world.er", 0)
}

#[test]
fn parse_simple_if() -> Result<(), ()> {
    expect_success("tests/simple_if.er", 0)
}

#[test]
fn parse_stream() -> Result<(), ()> {
    expect_success("tests/stream.er", 0)
}

#[test]
fn parse_test1_basic_syntax() -> Result<(), ()> {
    expect_success("tests/test1_basic_syntax.er", 0)
}

#[test]
fn parse_test2_advanced_syntax() -> Result<(), ()> {
    expect_success("tests/test2_advanced_syntax.er", 0)
}

#[test]
fn parse_stack() -> Result<(), ()> {
    expect_failure("tests/stack.er", 0, 2)
}

#[test]
fn parse_str_literal() -> Result<(), ()> {
    expect_failure("tests/failed_str_lit.er", 0, 2)
}

#[test]
fn exec_invalid_chunk_prs_err() -> Result<(), ()> {
    expect_failure("tests/invalid_chunk.er", 0, 62)
}

#[test]
fn exec_warns() -> Result<(), ()> {
    expect_success("tests/warns.er", 1)
}

fn _parse_test_from_code(
    file_path: &'static str,
) -> Result<ParseWarnings, ErrorArtifact<ParserRunnerErrors>> {
    let input = Input::file(file_path.into());
    let cfg = ErgConfig {
        input: input.clone(),
        py_server_timeout: 100,
        ..ErgConfig::default()
    };
    let lexer = Lexer::new(input.clone());
    let mut parser = ParserRunner::new(cfg);
    match parser.parse_token_stream(lexer.lex().map_err(|errs| {
        ErrorArtifact::new(
            ParserRunnerErrors::empty(),
            ParserRunnerErrors::convert(&input, errs),
        )
    })?) {
        Ok(artifact) => {
            if DEBUG_MODE {
                println!("{}", artifact.ast);
            }
            Ok(artifact.warns)
        }
        Err(artifact) => {
            if DEBUG_MODE {
                artifact.warns.write_all_stderr();
                artifact.errors.write_all_stderr();
            }
            Err(ErrorArtifact::new(artifact.warns, artifact.errors))
        }
    }
}

fn parse_test_from_code(
    file_path: &'static str,
) -> Result<ParseWarnings, ErrorArtifact<ParserRunnerErrors>> {
    exec_new_thread(move || _parse_test_from_code(file_path), file_path)
}

fn expect_success(file_path: &'static str, num_warns: usize) -> Result<(), ()> {
    match parse_test_from_code(file_path) {
        Ok(warns) => {
            if warns.len() == num_warns {
                Ok(())
            } else {
                println!(
                    "err: number of warnings is not {num_warns} but {}",
                    warns.len()
                );
                Err(())
            }
        }
        Err(_) => Err(()),
    }
}

fn expect_failure(file_path: &'static str, num_warns: usize, num_errs: usize) -> Result<(), ()> {
    match parse_test_from_code(file_path) {
        Ok(_) => Err(()),
        Err(eart) => match (eart.errors.len() == num_errs, eart.warns.len() == num_warns) {
            (true, true) => Ok(()),
            (true, false) => {
                println!(
                    "err: number of warnings is not {num_warns} but {}",
                    eart.warns.len()
                );
                Err(())
            }
            (false, true) => {
                println!(
                    "err: number of errors is not {num_errs} but {}",
                    eart.errors.len()
                );
                Err(())
            }
            (false, false) => {
                println!(
                    "err: number of warnings is not {num_warns} but {}",
                    eart.warns.len()
                );
                println!(
                    "err: number of errors is not {num_errs} but {}",
                    eart.errors.len()
                );
                Err(())
            }
        },
    }
}

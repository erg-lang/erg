extern crate erg_common;
extern crate erg_compiler;
extern crate erg_parser;

use std::process;
use std::thread;

use erg_common::config::ErgConfig;
use erg_common::traits::Runnable;

use erg_compiler::check::Checker;
use erg_compiler::lower::ASTLowerer;
use erg_compiler::Compiler;

use erg_parser::lex::LexerRunner;
use erg_parser::ParserRunner;

use erg_type::deserialize::Deserializer;

fn run() {
    let cfg = ErgConfig::parse();
    match cfg.mode {
        "lex" => {
            LexerRunner::run(cfg);
        }
        "parse" => {
            ParserRunner::run(cfg);
        }
        "lower" => {
            ASTLowerer::run(cfg);
        }
        "check" => {
            Checker::run(cfg);
        }
        "compile" | "exec" => {
            Compiler::run(cfg);
        }
        "read" => {
            Deserializer::run(cfg);
        }
        other => {
            println!("invalid mode: {other}");
            process::exit(1);
        }
    }
}

fn main() {
    if cfg!(windows) {
        const STACK_SIZE: usize = 4 * 1024 * 1024;

        let child = thread::Builder::new()
            .stack_size(STACK_SIZE)
            .spawn(run)
            .unwrap();

        // Wait for thread to join
        child.join().unwrap();
    } else {
        run();
    }
}

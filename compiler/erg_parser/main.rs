extern crate erg_common;
extern crate erg_parser;

use std::process;

use erg_common::config::ErgConfig;
use erg_common::spawn::exec_new_thread;
use erg_common::traits::Runnable;

use erg_parser::build_ast::ASTBuilder;
use erg_parser::lex::LexerRunner;
use erg_parser::ParserRunner;

fn run() {
    let cfg = ErgConfig::parse();
    match cfg.mode {
        "lex" => {
            LexerRunner::run(cfg);
        }
        "parse" => {
            ParserRunner::run(cfg);
        }
        "desugar" | "exec" => {
            ASTBuilder::run(cfg);
        }
        other => {
            println!("invalid mode: {other}");
            process::exit(1);
        }
    }
}

fn main() {
    exec_new_thread(run);
}

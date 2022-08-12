extern crate erg_common;
extern crate erg_parser;

use std::process;

use erg_common::config::ErgConfig;
use erg_common::traits::Runnable;

use erg_parser::lex::LexerRunner;
use erg_parser::ParserRunner;

fn main() {
    let cfg = ErgConfig::parse();
    match cfg.mode {
        "lex" => {
            LexerRunner::run(cfg);
        }
        "parse" | "exec" => {
            ParserRunner::run(cfg);
        }
        other => {
            println!("invalid mode: {other}");
            process::exit(1);
        }
    }
}

extern crate common;
extern crate parser;

use std::process;

use common::config::ErgConfig;
use common::traits::Runnable;

use parser::lex::LexerRunner;
use parser::ParserRunner;

fn main() {
    let cfg = ErgConfig::parse();
    match cfg.mode {
        "lex" => { LexerRunner::run(cfg); }
        "parse" | "exec" => { ParserRunner::run(cfg); }
        other => {
            println!("invalid mode: {other}");
            process::exit(1);
        }
    }
}

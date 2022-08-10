extern crate common;
extern crate compiler;
extern crate parser;

use std::process;

use common::deserialize::Deserializer;
use common::config::{ErgConfig};
use common::traits::Runnable;

use compiler::Compiler;

use parser::lex::LexerRunner;
use parser::ParserRunner;

fn main() {
    let cfg = ErgConfig::parse();
    match cfg.mode {
        "lex" => { LexerRunner::run(cfg); }
        "parse" => { ParserRunner::run(cfg); }
        "compile" | "exec" => { Compiler::run(cfg); }
        "read" => { Deserializer::run(cfg); }
        other => {
            println!("invalid mode: {other}");
            process::exit(1);
        }
    }
}

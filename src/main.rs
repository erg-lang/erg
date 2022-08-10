extern crate compiler;
extern crate parser;
extern crate erg;

use std::process;

use common::config::{ErgConfig};
use common::deserialize::Deserializer;
use common::traits::Runnable;

use parser::lex::LexerRunner;
use parser::ParserRunner;

use compiler::Compiler;

use erg::dummy::DummyVM;

fn main() {
    let cfg = ErgConfig::parse();
    match cfg.mode {
        "lex" => { LexerRunner::run(cfg); }
        "parse" => { ParserRunner::run(cfg); }
        "compile" => { Compiler::run(cfg); }
        "exec" => { DummyVM::run(cfg); }
        "read" => { Deserializer::run(cfg); }
        other => {
            eprintln!("invalid mode: {other}");
            process::exit(1);
        }
    }
}

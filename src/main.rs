extern crate erg;
extern crate erg_compiler;
extern crate erg_parser;

use std::process;

use erg_common::config::ErgConfig;
use erg_common::deserialize::Deserializer;
use erg_common::traits::Runnable;

use erg_parser::lex::LexerRunner;
use erg_parser::ParserRunner;

use erg_compiler::Compiler;

use erg::dummy::DummyVM;

fn main() {
    let cfg = ErgConfig::parse();
    match cfg.mode {
        "lex" => {
            LexerRunner::run(cfg);
        }
        "parse" => {
            ParserRunner::run(cfg);
        }
        "compile" => {
            Compiler::run(cfg);
        }
        "exec" => {
            DummyVM::run(cfg);
        }
        "read" => {
            Deserializer::run(cfg);
        }
        other => {
            eprintln!("invalid mode: {other}");
            process::exit(1);
        }
    }
}

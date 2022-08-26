extern crate erg_common;
extern crate erg_compiler;
extern crate erg_parser;

use std::process;

use erg_common::config::ErgConfig;
use erg_common::traits::Runnable;

use erg_compiler::Compiler;

use erg_parser::lex::LexerRunner;
use erg_parser::ParserRunner;

use erg_type::deserialize::Deserializer;

fn main() {
    let cfg = ErgConfig::parse();
    match cfg.mode {
        "lex" => {
            LexerRunner::run(cfg);
        }
        "parse" => {
            ParserRunner::run(cfg);
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

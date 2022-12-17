extern crate erg_common;
extern crate erg_compiler;
extern crate erg_parser;

use std::process;

use erg_common::config::ErgConfig;
use erg_common::spawn::exec_new_thread;
use erg_common::traits::Runnable;

use erg_compiler::build_hir::HIRBuilder;
use erg_compiler::lower::ASTLowerer;
use erg_compiler::transpile::Transpiler;
use erg_compiler::ty::deserialize::Deserializer;
use erg_compiler::Compiler;

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
        "lower" => {
            ASTLowerer::run(cfg);
        }
        "check" => {
            HIRBuilder::run(cfg);
        }
        "transpile" => {
            Transpiler::run(cfg);
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
    exec_new_thread(run);
}

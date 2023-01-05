extern crate erg_common;
extern crate erg_compiler;
extern crate erg_parser;

use std::process;

use erg_common::config::{ErgConfig, ErgMode::*};
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
        Lex => {
            LexerRunner::run(cfg);
        }
        Parse => {
            ParserRunner::run(cfg);
        }
        TypeCheck => {
            ASTLowerer::run(cfg);
        }
        FullCheck => {
            HIRBuilder::run(cfg);
        }
        Transpile => {
            Transpiler::run(cfg);
        }
        Compile | Execute => {
            Compiler::run(cfg);
        }
        Read => {
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

extern crate erg;
extern crate erg_compiler;
extern crate erg_parser;

use std::process;

use erg_common::config::ErgConfig;
use erg_common::spawn::exec_new_thread;
use erg_common::traits::Runnable;

use erg_parser::build_ast::ASTBuilder;
use erg_parser::lex::LexerRunner;
use erg_parser::ParserRunner;

use erg_compiler::build_hir::HIRBuilder;
use erg_compiler::lower::ASTLowerer;
use erg_compiler::transpile::Transpiler;
use erg_compiler::ty::deserialize::Deserializer;
use erg_compiler::Compiler;

use erg::DummyVM;

fn run() {
    let cfg = ErgConfig::parse();
    match cfg.mode {
        "lex" => {
            LexerRunner::run(cfg);
        }
        "parse" => {
            ParserRunner::run(cfg);
        }
        "desugar" => {
            ASTBuilder::run(cfg);
        }
        "lower" => {
            ASTLowerer::run(cfg);
        }
        "check" => {
            HIRBuilder::run(cfg);
        }
        "compile" => {
            Compiler::run(cfg);
        }
        "transpile" => {
            Transpiler::run(cfg);
        }
        "exec" => {
            DummyVM::run(cfg);
        }
        "read" => {
            Deserializer::run(cfg);
        }
        "language-server" => {
            #[cfg(feature = "els")]
            {
                use els::ErgLanguageServer;
                let mut server = ErgLanguageServer::new(cfg);
                server.run().unwrap();
            }
            #[cfg(not(feature = "els"))]
            {
                eprintln!("This version of the build does not support language server mode");
                process::exit(1);
            }
        }
        other => {
            eprintln!("invalid mode: {other}");
            process::exit(1);
        }
    }
}

fn main() {
    exec_new_thread(run);
}

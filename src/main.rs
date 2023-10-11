extern crate erg;
extern crate erg_compiler;
extern crate erg_parser;

use erg_common::config::{ErgConfig, ErgMode::*};
use erg_common::spawn::exec_new_thread;
use erg_common::traits::{ExitStatus, Runnable};

use erg_compiler::build_package::{FullPackageBuilder, PackageTypeChecker};
use erg_parser::lex::LexerRunner;
use erg_parser::ParserRunner;

use erg_compiler::transpile::Transpiler;
use erg_compiler::ty::deserialize::Deserializer;
use erg_compiler::{ASTBuilder, Compiler};

use erg::DummyVM;

fn run() {
    let cfg = ErgConfig::parse();
    let stat = match cfg.mode {
        Lex => LexerRunner::run(cfg),
        Parse => ParserRunner::run(cfg),
        Desugar => ASTBuilder::run(cfg),
        TypeCheck => PackageTypeChecker::run(cfg),
        FullCheck => FullPackageBuilder::run(cfg),
        Compile => Compiler::run(cfg),
        Transpile => Transpiler::run(cfg),
        Execute => DummyVM::run(cfg),
        Read => Deserializer::run(cfg),
        LanguageServer => {
            #[cfg(feature = "els")]
            {
                use els::ErgLanguageServer;
                let mut server = ErgLanguageServer::new(cfg, None);
                server.run().unwrap();
                ExitStatus::OK
            }
            #[cfg(not(feature = "els"))]
            {
                eprintln!("This version of the build does not support language server mode");
                ExitStatus::ERR1
            }
        }
    };
    std::process::exit(stat.code);
}

fn main() {
    exec_new_thread(run, "erg");
}

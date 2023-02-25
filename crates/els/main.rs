mod code_action;
mod code_lens;
mod command;
mod completion;
mod definition;
mod diagnostics;
mod file_cache;
mod hir_visitor;
mod hover;
mod inlay_hint;
mod message;
mod references;
mod rename;
mod semantic;
mod server;
mod sig_help;
mod util;

use erg_common::config::ErgConfig;

fn main() {
    let cfg = ErgConfig::default();
    let mut server = server::ErgLanguageServer::new(cfg);
    server.run().unwrap();
}

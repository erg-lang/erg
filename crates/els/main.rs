mod call_hierarchy;
mod channels;
mod code_action;
mod code_lens;
mod command;
mod completion;
mod definition;
mod diagnostics;
mod diff;
mod doc_highlight;
mod file_cache;
mod folding_range;
mod hir_visitor;
mod hover;
mod implementation;
mod inlay_hint;
mod message;
mod references;
mod rename;
mod scheduler;
mod selection_range;
mod semantic;
mod server;
mod sig_help;
mod symbol;
mod type_definition;
mod util;

use erg_common::config::ErgConfig;

fn main() {
    let cfg = ErgConfig::default();
    let server = server::ErgLanguageServer::new(cfg, None);
    server.run();
}

mod completion;
mod definition;
mod diagnostics;
mod hir_visitor;
mod hover;
mod message;
mod references;
mod rename;
mod server;
mod util;

use erg_common::config::ErgConfig;

fn main() {
    let cfg = ErgConfig::default();
    let mut server = server::ErgLanguageServer::new(cfg);
    server.run().unwrap();
}

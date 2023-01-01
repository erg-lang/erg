mod hir_visitor;
mod message;
mod server;

use erg_common::config::ErgConfig;

fn main() {
    let cfg = ErgConfig::default();
    let mut server = server::ErgLanguageServer::new(cfg);
    server.run().unwrap();
}

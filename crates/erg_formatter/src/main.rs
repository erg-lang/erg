mod format;
mod transform;

use std::process;

use erg_common::config::ErgConfig;
use erg_common::spawn::exec_new_thread;
use erg_common::traits::Runnable;

use crate::format::Formatter;

fn run() {
    let cfg = ErgConfig::parse();
    let stat = Formatter::run(cfg);
    process::exit(stat.code);
}

fn main() {
    exec_new_thread(run, "ergfmt")
}

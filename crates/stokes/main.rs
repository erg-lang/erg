use erg_common::config::ErgConfig;

mod generator;

fn main() {
    let cfg = ErgConfig::parse();
    let mut gen = generator::HTMLGenerator::new(cfg);
    gen.generate_builtin();
}

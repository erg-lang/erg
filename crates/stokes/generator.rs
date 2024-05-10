use erg_common::config::ErgConfig;

#[derive(Debug)]
pub struct HTMLGenerator {
    pub config: ErgConfig,
}

impl HTMLGenerator {
    pub fn new(config: ErgConfig) -> Self {
        HTMLGenerator { config }
    }

    pub fn generate(&mut self) {}
}

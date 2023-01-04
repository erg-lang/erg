use erg_common::config::ErgConfig;

use crate::index::SharedModuleIndex;
use crate::mod_cache::SharedModuleCache;

#[derive(Debug, Clone, Default)]
pub struct SharedCompilerResource {
    pub(crate) mod_cache: SharedModuleCache,
    pub(crate) py_mod_cache: SharedModuleCache,
    pub(crate) index: SharedModuleIndex,
}

impl SharedCompilerResource {
    pub fn new(cfg: ErgConfig) -> Self {
        Self {
            mod_cache: SharedModuleCache::new(cfg.copy()),
            py_mod_cache: SharedModuleCache::new(cfg),
            index: SharedModuleIndex::new(),
        }
    }
}

use erg_common::config::ErgConfig;

use super::cache::SharedModuleCache;
#[cfg(feature = "els")]
use super::index::SharedModuleIndex;

#[derive(Debug, Clone, Default)]
pub struct SharedCompilerResource {
    pub mod_cache: SharedModuleCache,
    pub py_mod_cache: SharedModuleCache,
    #[cfg(feature = "els")]
    pub index: SharedModuleIndex,
}

impl SharedCompilerResource {
    pub fn new(cfg: ErgConfig) -> Self {
        Self {
            mod_cache: SharedModuleCache::new(cfg.copy()),
            py_mod_cache: SharedModuleCache::new(cfg),
            #[cfg(feature = "els")]
            index: SharedModuleIndex::new(),
        }
    }

    pub fn clear_all(&self) {
        self.mod_cache.initialize();
        self.py_mod_cache.initialize();
        #[cfg(feature = "els")]
        self.index.initialize();
    }
}

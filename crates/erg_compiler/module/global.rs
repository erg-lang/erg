use erg_common::config::ErgConfig;
use erg_common::pathutil::NormalizedPathBuf;
use erg_common::shared::MappedRwLockReadGuard;
use erg_common::spawn::safe_yield;

use crate::context::{Context, ModuleContext};

use super::cache::{ModuleEntry, SharedGeneralizationCache, SharedModuleCache};
use super::errors::{SharedCompileErrors, SharedCompileWarnings};
use super::graph::SharedModuleGraph;
use super::impls::SharedTraitImpls;
use super::index::SharedModuleIndex;
use super::promise::SharedPromises;

fn try_forever<T, F: FnMut() -> Result<T, ()>>(mut f: F) -> T {
    loop {
        if let Ok(res) = f() {
            return res;
        }
        safe_yield();
    }
}

#[derive(Debug, Clone, Default)]
pub struct SharedCompilerResource {
    pub mod_cache: SharedModuleCache,
    pub py_mod_cache: SharedModuleCache,
    pub index: SharedModuleIndex,
    pub graph: SharedModuleGraph,
    /// K: name of a trait, V: (type, monomorphised trait that the type implements)
    /// K: トレイトの名前, V: (型, その型が実装する単相化トレイト)
    /// e.g. { "Named": [(Type, Named), (Func, Named), ...], "Add": [(Nat, Add(Nat)), (Int, Add(Int)), ...], ... }
    pub trait_impls: SharedTraitImpls,
    pub promises: SharedPromises,
    pub errors: SharedCompileErrors,
    pub warns: SharedCompileWarnings,
    pub gen_cache: SharedGeneralizationCache,
}

impl SharedCompilerResource {
    /// Initialize the shared compiler resource.
    /// This API is normally called only once throughout the compilation phase.
    pub fn new(cfg: ErgConfig) -> Self {
        let graph = SharedModuleGraph::new();
        let self_ = Self {
            mod_cache: SharedModuleCache::new(),
            py_mod_cache: SharedModuleCache::new(),
            index: SharedModuleIndex::new(),
            graph: graph.clone(),
            trait_impls: SharedTraitImpls::new(),
            promises: SharedPromises::new(graph, NormalizedPathBuf::from(cfg.input.path())),
            errors: SharedCompileErrors::new(),
            warns: SharedCompileWarnings::new(),
            gen_cache: SharedGeneralizationCache::new(),
        };
        Context::init_builtins(cfg, self_.clone());
        self_
    }

    pub fn inherit<P: Into<NormalizedPathBuf>>(&self, path: P) -> Self {
        let mut _self = self.clone();
        _self.promises.path = path.into();
        _self
    }

    /// Clear all but builtin modules
    pub fn clear_all(&self) {
        self.mod_cache.initialize();
        self.py_mod_cache.initialize();
        self.index.initialize();
        self.graph.initialize();
        self.trait_impls.initialize();
        self.promises.initialize();
        self.errors.clear();
        self.warns.clear();
    }

    /// Clear all information about the module.
    /// Graph information is not cleared (due to ELS).
    pub fn clear(&self, path: &NormalizedPathBuf) -> Option<ModuleEntry> {
        let mut old = None;
        for child in self.graph.children(path) {
            self.clear(&child);
        }
        try_forever(|| {
            if let Some(ent) = self.mod_cache.try_remove(path)? {
                old = Some(ent);
            }
            Ok(())
        });
        try_forever(|| {
            if let Some(ent) = self.py_mod_cache.try_remove(path)? {
                old = Some(ent);
            }
            Ok(())
        });
        self.index.remove_path(path);
        // self.graph.remove(path);
        self.trait_impls.remove_by_path(path);
        self.promises.remove(path);
        self.errors.remove(path);
        self.warns.remove(path);
        old
    }

    pub fn clear_path(&self, path: &NormalizedPathBuf) {
        try_forever(|| self.mod_cache.try_remove(path));
        try_forever(|| self.py_mod_cache.try_remove(path));
        self.index.remove_path(path);
        // self.graph.remove(path);
        self.trait_impls.remove_by_path(path);
        self.promises.remove(path);
        self.errors.remove(path);
        self.warns.remove(path);
    }

    pub fn rename_path(&self, old: &NormalizedPathBuf, new: NormalizedPathBuf) {
        self.mod_cache.rename_path(old, new.clone());
        self.py_mod_cache.rename_path(old, new.clone());
        self.index.rename_path(old, new.clone());
        self.trait_impls.rename_path(old, new.clone());
        self.graph.rename_path(old, new.clone());
        self.promises.rename(old, new);
    }

    pub fn insert_module(&self, path: NormalizedPathBuf, entry: ModuleEntry) {
        if path.to_string_lossy().ends_with(".d.er") {
            self.py_mod_cache.insert(path, entry);
        } else {
            self.mod_cache.insert(path, entry);
        }
    }

    /// This is intended to be called from a language server, etc.,
    /// this blocks until it gets a lock.
    pub fn remove_module(&self, path: &std::path::Path) -> Option<ModuleEntry> {
        if path.to_string_lossy().ends_with(".d.er") {
            try_forever(|| self.py_mod_cache.try_remove(path))
        } else {
            try_forever(|| self.mod_cache.try_remove(path))
        }
    }

    pub fn get_module(&self, path: &std::path::Path) -> Option<MappedRwLockReadGuard<ModuleEntry>> {
        if path.to_string_lossy().ends_with(".d.er") {
            self.py_mod_cache.get(path)
        } else {
            self.mod_cache.get(path)
        }
    }

    pub fn raw_ref_ctx_with_timeout(
        &self,
        path: &std::path::Path,
        timeout: std::time::Duration,
    ) -> Option<&ModuleContext> {
        if path.to_string_lossy().ends_with(".d.er") {
            self.py_mod_cache.raw_ref_ctx_with_timeout(path, timeout)
        } else {
            self.mod_cache.raw_ref_ctx_with_timeout(path, timeout)
        }
    }

    pub fn raw_ref_builtins_ctx(&self) -> Option<&ModuleContext> {
        self.mod_cache.raw_ref_builtins_ctx()
    }

    pub fn raw_modules(&self) -> impl Iterator<Item = &ModuleEntry> {
        self.mod_cache
            .raw_values()
            .chain(self.py_mod_cache.raw_values())
    }

    pub fn raw_path_and_modules(&self) -> impl Iterator<Item = (&NormalizedPathBuf, &ModuleEntry)> {
        self.mod_cache
            .raw_iter()
            .chain(self.py_mod_cache.raw_iter())
    }
}

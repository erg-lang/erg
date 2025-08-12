use std::borrow::Borrow;
use std::fmt;
use std::hash::Hash;

use erg_common::dict::Dict;
use erg_common::pathutil::NormalizedPathBuf;
use erg_common::set::Set;
use erg_common::shared::{
    MappedRwLockReadGuard, MappedRwLockWriteGuard, RwLockReadGuard, RwLockWriteGuard, Shared,
};
use erg_common::Str;

use crate::context::TraitImpl;

/// Caches checked modules.
/// In addition to being queried here when re-imported, it is also used when linking
/// (Erg links all scripts defined in erg and outputs them to a single pyc file).
#[derive(Debug, Default)]
pub struct TraitImpls {
    /// * key: trait qualified name
    /// * value: set of trait impls
    cache: Dict<Str, Set<TraitImpl>>,
}

impl fmt::Display for TraitImpls {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TraitImpls {{")?;
        for (name, impls) in self.cache.iter() {
            writeln!(f, "{name}: {impls}, ")?;
        }
        write!(f, "}}")
    }
}

impl TraitImpls {
    pub fn new() -> Self {
        Self { cache: Dict::new() }
    }

    pub fn get<P: Eq + Hash + ?Sized>(&self, path: &P) -> Option<&Set<TraitImpl>>
    where
        Str: Borrow<P>,
    {
        self.cache.get(path)
    }

    pub fn get_mut<Q: Eq + Hash + ?Sized>(&mut self, path: &Q) -> Option<&mut Set<TraitImpl>>
    where
        Str: Borrow<Q>,
    {
        self.cache.get_mut(path)
    }

    pub fn register(&mut self, name: Str, impls: Set<TraitImpl>) {
        self.cache.insert(name, impls);
    }

    pub fn remove<Q: Eq + Hash + ?Sized>(&mut self, path: &Q) -> Option<Set<TraitImpl>>
    where
        Str: Borrow<Q>,
    {
        self.cache.remove(path)
    }

    pub fn remove_by_path(&mut self, path: &NormalizedPathBuf) {
        for impls in self.cache.values_mut() {
            impls.retain(|impl_| impl_.declared_in.as_ref() != Some(path));
        }
    }

    pub fn rename<Q: Eq + Hash + ?Sized>(&mut self, old: &Q, new: Str)
    where
        Str: Borrow<Q>,
    {
        if let Some(impls) = self.remove(old) {
            self.register(new, impls);
        }
    }

    pub fn rename_path(&mut self, old: &NormalizedPathBuf, new: NormalizedPathBuf) {
        for impls in self.cache.values_mut() {
            impls.inplace_map(|mut impl_| {
                if impl_.declared_in.as_ref() == Some(old) {
                    impl_.declared_in = Some(new.clone());
                    impl_
                } else {
                    impl_
                }
            });
        }
    }

    pub fn initialize(&mut self) {
        for impls in self.cache.values_mut() {
            impls.retain(|impl_| impl_.declared_in.is_none());
        }
        self.cache.retain(|_, impls| !impls.is_empty());
    }
}

#[derive(Debug, Clone, Default)]
pub struct SharedTraitImpls(Shared<TraitImpls>);

impl fmt::Display for SharedTraitImpls {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Shared{}", self.0)
    }
}

impl SharedTraitImpls {
    pub fn new() -> Self {
        Self(Shared::new(TraitImpls::new()))
    }

    pub fn get<Q: Eq + Hash + ?Sized>(
        &self,
        path: &Q,
    ) -> Option<MappedRwLockReadGuard<'_, Set<TraitImpl>>>
    where
        Str: Borrow<Q>,
    {
        RwLockReadGuard::try_map(self.0.borrow(), |tis| tis.get(path)).ok()
    }

    pub fn get_mut<Q: Eq + Hash + ?Sized>(
        &self,
        path: &Q,
    ) -> Option<MappedRwLockWriteGuard<'_, Set<TraitImpl>>>
    where
        Str: Borrow<Q>,
    {
        RwLockWriteGuard::try_map(self.0.borrow_mut(), |tis| tis.get_mut(path)).ok()
    }

    pub fn register(&self, name: Str, impls: Set<TraitImpl>) {
        self.0.borrow_mut().register(name, impls);
    }

    pub fn remove<Q: Eq + Hash + ?Sized>(&self, qual_name: &Q) -> Option<Set<TraitImpl>>
    where
        Str: Borrow<Q>,
    {
        self.0.borrow_mut().remove(qual_name)
    }

    pub fn remove_by_path(&self, path: &NormalizedPathBuf) {
        self.0.borrow_mut().remove_by_path(path);
    }

    pub fn rename<Q: Eq + Hash + ?Sized>(&self, old: &Q, new: Str)
    where
        Str: Borrow<Q>,
    {
        self.0.borrow_mut().rename(old, new);
    }

    pub fn rename_path(&self, old: &NormalizedPathBuf, new: NormalizedPathBuf) {
        self.0.borrow_mut().rename_path(old, new);
    }

    pub fn ref_inner(&self) -> MappedRwLockReadGuard<'_, Dict<Str, Set<TraitImpl>>> {
        RwLockReadGuard::map(self.0.borrow(), |tis| &tis.cache)
    }

    pub fn initialize(&self) {
        self.0.borrow_mut().initialize();
    }
}

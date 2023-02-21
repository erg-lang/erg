use std::borrow::Borrow;
use std::fmt;
use std::hash::Hash;

use erg_common::dict::Dict;
use erg_common::set::Set;
use erg_common::shared::Shared;
use erg_common::Str;

use crate::context::TraitImpl;

/// Caches checked modules.
/// In addition to being queried here when re-imported, it is also used when linking
/// (Erg links all scripts defined in erg and outputs them to a single pyc file).
#[derive(Debug, Default)]
pub struct TraitImpls {
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

    pub fn initialize(&mut self) {
        self.cache.clear();
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

    pub fn get<Q: Eq + Hash + ?Sized>(&self, path: &Q) -> Option<&Set<TraitImpl>>
    where
        Str: Borrow<Q>,
    {
        let ref_ = unsafe { self.0.as_ptr().as_ref().unwrap() };
        ref_.get(path)
    }

    pub fn get_mut<Q: Eq + Hash + ?Sized>(&self, path: &Q) -> Option<&mut Set<TraitImpl>>
    where
        Str: Borrow<Q>,
    {
        let ref_ = unsafe { self.0.as_ptr().as_mut().unwrap() };
        ref_.get_mut(path)
    }

    pub fn register(&self, name: Str, impls: Set<TraitImpl>) {
        self.0.borrow_mut().register(name, impls);
    }

    pub fn remove<Q: Eq + Hash + ?Sized>(&self, path: &Q) -> Option<Set<TraitImpl>>
    where
        Str: Borrow<Q>,
    {
        self.0.borrow_mut().remove(path)
    }

    pub fn keys(&self) -> impl Iterator<Item = Str> {
        let ref_ = unsafe { self.0.as_ptr().as_ref().unwrap() };
        ref_.cache.keys().cloned()
    }

    pub fn initialize(&self) {
        self.0.borrow_mut().initialize();
    }
}

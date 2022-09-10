use std::borrow::Borrow;
use std::cell::RefCell;
use std::hash::Hash;
use std::thread::LocalKey;

use erg_common::dict::Dict;

use erg_type::Type;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SubtypePair {
    pub sub: Type,
    pub sup: Type,
}

impl SubtypePair {
    pub const fn new(sub: Type, sup: Type) -> Self {
        Self { sub, sup }
    }
}

#[derive(Debug, Default)]
pub struct TypeCmpCache {
    cache: Dict<SubtypePair, bool>,
}

impl TypeCmpCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get<Q: Eq + Hash>(&self, pair: &Q) -> Option<bool>
    where
        SubtypePair: Borrow<Q>,
    {
        self.cache.get(pair).copied()
    }

    pub fn register(&mut self, pair: SubtypePair, b: bool) {
        self.cache.insert(pair, b);
    }
}

thread_local! {
    static TYPE_CACHE: RefCell<TypeCmpCache> = RefCell::new(TypeCmpCache::default());
}

#[derive(Debug)]
pub struct GlobalTypeCmpCache(LocalKey<RefCell<TypeCmpCache>>);

pub static GLOBAL_TYPE_CACHE: GlobalTypeCmpCache = GlobalTypeCmpCache(TYPE_CACHE);

impl GlobalTypeCmpCache {
    pub fn get<Q: Eq + Hash>(&'static self, pair: &Q) -> Option<bool>
    where
        SubtypePair: Borrow<Q>,
    {
        self.0.with(|s| s.borrow().get(pair))
    }

    pub fn register(&'static self, pair: SubtypePair, b: bool) {
        self.0.with(|s| s.borrow_mut().register(pair, b));
    }
}
